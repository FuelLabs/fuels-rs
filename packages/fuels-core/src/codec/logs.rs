use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
    iter::FilterMap,
};

use fuel_tx::{ContractId, Receipt};

use crate::{
    codec::{ABIDecoder, DecoderConfig},
    traits::{Parameterize, Tokenizable},
    types::errors::{error, Error, Result},
};

#[derive(Clone)]
pub struct LogFormatter {
    formatter: fn(DecoderConfig, &[u8]) -> Result<String>,
    type_id: TypeId,
}

impl LogFormatter {
    pub fn new<T: Tokenizable + Parameterize + Debug + 'static>() -> Self {
        Self {
            formatter: Self::format_log::<T>,
            type_id: TypeId::of::<T>(),
        }
    }

    fn format_log<T: Parameterize + Tokenizable + Debug>(
        decoder_config: DecoderConfig,
        bytes: &[u8],
    ) -> Result<String> {
        #[cfg(not(feature = "experimental"))]
        Self::can_decode_log_with_type::<T>()?;

        let token = ABIDecoder::new(decoder_config).decode(&T::param_type(), bytes)?;

        Ok(format!("{:?}", T::from_token(token)?))
    }

    #[cfg(not(feature = "experimental"))]
    fn can_decode_log_with_type<T: Parameterize>() -> Result<()> {
        use crate::types::param_types::ParamType;

        match T::param_type() {
            // String slices cannot be decoded from logs as they are encoded as ptr, len
            // TODO: Once https://github.com/FuelLabs/sway/issues/5110 is resolved we can remove this
            ParamType::StringSlice => Err(error!(
                Codec,
                "string slices cannot be decoded from logs. Convert the slice to `str[N]` with `__to_str_array`"
            )),
            _ => Ok(()),
        }
    }

    pub fn can_handle_type<T: Tokenizable + Parameterize + 'static>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    pub fn format(&self, decoder_config: DecoderConfig, bytes: &[u8]) -> Result<String> {
        (self.formatter)(decoder_config, bytes)
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
    log_formatters: HashMap<LogId, LogFormatter>,
    decoder_config: DecoderConfig,
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
    pub fn new(log_formatters: HashMap<LogId, LogFormatter>) -> Self {
        Self {
            log_formatters,
            decoder_config: Default::default(),
        }
    }

    pub fn set_decoder_config(&mut self, decoder_config: DecoderConfig) -> &mut Self {
        self.decoder_config = decoder_config;
        self
    }

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
                    Codec,
                    "missing log formatter for log_id: `{:?}`, data: `{:?}`. \
                     Consider adding external contracts using `with_contracts()`",
                    log_id,
                    data
                )
            })
            .and_then(|log_formatter| log_formatter.format(self.decoder_config, data))
    }

    pub(crate) fn decode_last_log(&self, receipts: &[Receipt]) -> Result<String> {
        receipts
            .iter()
            .rev()
            .extract_log_id_and_data()
            .next()
            .ok_or_else(|| error!(Codec, "no receipts found for decoding last log"))
            .and_then(|(log_id, data)| self.format_log(&log_id, &data))
    }

    pub(crate) fn decode_last_two_logs(&self, receipts: &[Receipt]) -> Result<(String, String)> {
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
                Codec,
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
            .filter(|(_, log_formatter)| log_formatter.can_handle_type::<T>())
            .map(|(log_id, _)| log_id.clone())
            .collect();

        receipts
            .iter()
            .extract_log_id_and_data()
            .filter_map(|(log_id, bytes)| {
                target_ids.contains(&log_id).then(|| {
                    let token =
                        ABIDecoder::new(self.decoder_config).decode(&T::param_type(), &bytes)?;

                    T::from_token(token)
                })
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
            Receipt::LogData {
                rb,
                data: Some(data),
                id,
                ..
            } => Some((LogId(*id, *rb), data.clone())),
            Receipt::Log { ra, rb, id, .. } => Some((LogId(*id, *rb), ra.to_be_bytes().to_vec())),
            _ => None,
        })
    }
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
