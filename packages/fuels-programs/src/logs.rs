use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
    iter::FilterMap,
};

use fuel_abi_types::error_codes::{
    FAILED_ASSERT_EQ_SIGNAL, FAILED_ASSERT_SIGNAL, FAILED_REQUIRE_SIGNAL,
    FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL,
};
use fuel_tx::{ContractId, Receipt};
use fuels_core::{
    codec::try_from_bytes,
    traits::{Parameterize, Tokenizable},
    types::errors::{error, Error, Result},
};

#[derive(Clone)]
pub struct LogFormatter {
    formatter: fn(&[u8]) -> Result<String>,
    type_id: TypeId,
}

impl LogFormatter {
    pub fn new<T: Tokenizable + Parameterize + Debug + 'static>() -> Self {
        Self {
            formatter: Self::format_log::<T>,
            type_id: TypeId::of::<T>(),
        }
    }

    fn format_log<T: Parameterize + Tokenizable + Debug>(bytes: &[u8]) -> Result<String> {
        Ok(format!("{:?}", try_from_bytes::<T>(bytes)?))
    }

    pub fn can_handle_type<T: Tokenizable + Parameterize + 'static>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    pub fn format(&self, bytes: &[u8]) -> Result<String> {
        (self.formatter)(bytes)
    }
}

impl Debug for LogFormatter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogFormatter")
            .field("type_id", &self.type_id)
            .finish()
    }
}

/// Holds a unique log ID
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LogId(ContractId, u64);

/// Struct used to pass the log mappings from the Abigen
#[derive(Debug, Clone, Default)]
pub struct LogDecoder {
    /// A mapping of LogId and param-type
    pub log_formatters: HashMap<LogId, LogFormatter>,
}

#[derive(Debug)]
pub struct LogResult {
    pub results: Vec<Result<String>>,
}

impl LogResult {
    pub fn filter_succeeded(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter_map(|result| result.as_deref().ok())
            .collect()
    }

    pub fn filter_failed(&self) -> Vec<&Error> {
        self.results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .collect()
    }
}

impl LogDecoder {
    /// Get all logs results from the given receipts as `Result<String>`
    pub fn decode_logs(&self, receipts: &[Receipt]) -> LogResult {
        let results = receipts
            .iter()
            .extract_log_id_and_data()
            .map(|(log_id, data)| self.format_log(&log_id, &data))
            .collect();

        LogResult { results }
    }

    fn format_log(&self, log_id: &LogId, data: &[u8]) -> Result<String> {
        self.log_formatters
            .get(log_id)
            .ok_or_else(|| {
                error!(
                    InvalidData,
                    "missing log formatter for log_id: `{:?}`, data: `{:?}`. \
                     Consider adding external contracts with `set_contracts()`",
                    log_id,
                    data
                )
            })
            .and_then(|log_formatter| log_formatter.format(data))
    }

    fn decode_last_log(&self, receipts: &[Receipt]) -> Result<String> {
        receipts
            .iter()
            .rev()
            .extract_log_id_and_data()
            .next()
            .ok_or_else(|| error!(InvalidData, "No receipts found for decoding last log."))
            .and_then(|(log_id, data)| self.format_log(&log_id, &data))
    }

    fn decode_last_two_logs(&self, receipts: &[Receipt]) -> Result<(String, String)> {
        let res = receipts
            .iter()
            .rev()
            .extract_log_id_and_data()
            .map(|(log_id, data)| self.format_log(&log_id, &data))
            .take(2)
            .collect::<Result<Vec<_>>>();

        match res.as_deref() {
            Ok([rhs, lhs]) => Ok((lhs.to_string(), rhs.to_string())),
            Ok(some_slice) => Err(error!(
                InvalidData,
                "expected to have two logs. Found {}",
                some_slice.len()
            )),
            Err(_) => Err(res.expect_err("must be an error")),
        }
    }

    /// Get decoded logs with specific type from the given receipts.
    /// Note that this method returns the actual type and not a `String` representation.
    pub fn decode_logs_with_type<T: Tokenizable + Parameterize + 'static>(
        &self,
        receipts: &[Receipt],
    ) -> Result<Vec<T>> {
        let target_ids: HashSet<LogId> = self
            .log_formatters
            .iter()
            .filter_map(|(log_id, log_formatter)| {
                log_formatter.can_handle_type::<T>().then(|| log_id.clone())
            })
            .collect();

        receipts
            .iter()
            .extract_log_id_and_data()
            .filter_map(|(log_id, data)| {
                target_ids
                    .contains(&log_id)
                    .then_some(try_from_bytes(&data))
            })
            .collect()
    }

    pub fn merge(&mut self, log_decoder: LogDecoder) {
        self.log_formatters.extend(log_decoder.log_formatters);
    }
}

trait ExtractLogIdData {
    type Output: Iterator<Item = (LogId, Vec<u8>)>;
    fn extract_log_id_and_data(self) -> Self::Output;
}

impl<'a, I: Iterator<Item = &'a Receipt>> ExtractLogIdData for I {
    type Output = FilterMap<Self, fn(&Receipt) -> Option<(LogId, Vec<u8>)>>;
    fn extract_log_id_and_data(self) -> Self::Output {
        self.filter_map(|r| match r {
            Receipt::LogData { rb, data, id, .. } => Some((LogId(*id, *rb), data.clone())),
            Receipt::Log { ra, rb, id, .. } => Some((LogId(*id, *rb), ra.to_be_bytes().to_vec())),
            _ => None,
        })
    }
}

/// Map the provided `RevertTransactionError` based on the `revert_id`.
/// If applicable, decode the logged types from the receipt.
pub fn map_revert_error(mut err: Error, log_decoder: &LogDecoder) -> Error {
    if let Error::RevertTransactionError {
        revert_id,
        ref receipts,
        ref mut reason,
    } = err
    {
        match revert_id {
            FAILED_REQUIRE_SIGNAL => {
                *reason = log_decoder.decode_last_log(receipts).unwrap_or_else(|err| {
                    format!("failed to decode log from require revert: {err}")
                })
            }
            FAILED_ASSERT_EQ_SIGNAL => {
                *reason = match log_decoder.decode_last_two_logs(receipts) {
                    Ok((lhs, rhs)) => format!(
                        "assertion failed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                    ),
                    Err(err) => format!("failed to decode log from assert_eq revert: {err}"),
                };
            }
            FAILED_ASSERT_SIGNAL => *reason = "assertion failed.".into(),
            FAILED_SEND_MESSAGE_SIGNAL => *reason = "failed to send message.".into(),
            FAILED_TRANSFER_TO_ADDRESS_SIGNAL => *reason = "failed transfer to address.".into(),
            _ => {}
        }
    }
    err
}

pub fn log_formatters_lookup(
    log_id_log_formatter_pairs: Vec<(u64, LogFormatter)>,
    contract_id: ContractId,
) -> HashMap<LogId, LogFormatter> {
    log_id_log_formatter_pairs
        .into_iter()
        .map(|(id, log_formatter)| (LogId(contract_id, id), log_formatter))
        .collect()
}
