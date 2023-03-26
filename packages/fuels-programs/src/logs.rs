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
use fuels_core::try_from_bytes;
use fuels_types::{
    errors::{Error, Result},
    traits::{Parameterize, Tokenizable},
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

impl LogDecoder {
    /// Get all decoded logs from the given receipts as `String`
    pub fn get_logs(&self, receipts: &[Receipt]) -> Result<Vec<String>> {
        receipts
            .iter()
            .extract_log_id_and_data()
            .filter_map(|(log_id, data)| {
                self.log_formatters
                    .get(&log_id)
                    .map(|log_formatter| log_formatter.format(&data))
            })
            .collect()
    }

    /// Get decoded logs with specific type from the given receipts.
    /// Note that this method returns the actual type and not a `String` representation.
    pub fn get_logs_with_type<T: Tokenizable + Parameterize + 'static>(
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
            FAILED_REQUIRE_SIGNAL => *reason = decode_require_revert(log_decoder, receipts),
            FAILED_ASSERT_EQ_SIGNAL => *reason = decode_assert_eq_revert(log_decoder, receipts),
            FAILED_ASSERT_SIGNAL => *reason = "assertion failed.".into(),
            FAILED_SEND_MESSAGE_SIGNAL => *reason = "failed to send message.".into(),
            FAILED_TRANSFER_TO_ADDRESS_SIGNAL => *reason = "failed transfer to address.".into(),
            _ => {}
        }
    }
    err
}

fn decode_require_revert(log_decoder: &LogDecoder, receipts: &[Receipt]) -> String {
    log_decoder
        .get_logs(receipts)
        .ok()
        .and_then(|logs| logs.last().cloned())
        .unwrap_or_else(|| "failed to decode log from require revert".to_string())
}

fn decode_assert_eq_revert(log_decoder: &LogDecoder, receipts: &[Receipt]) -> String {
    log_decoder
        .get_logs(receipts)
        .ok()
        .and_then(|logs| {
            if let [.., lhs, rhs] = logs.as_slice() {
                return Some(format!(
                    "assertion failed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                ));
            }
            None
        })
        .unwrap_or_else(|| "failed to decode logs from assert_eq revert".to_string())
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
