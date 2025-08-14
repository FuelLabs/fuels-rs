use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
    iter::FilterMap,
};

#[derive(Debug, Clone)]
pub struct ErrorDetails {
    pub(crate) pkg: String,
    pub(crate) file: String,
    pub(crate) line: u64,
    pub(crate) column: u64,
    pub(crate) log_id: Option<String>,
    pub(crate) msg: Option<String>,
}

impl ErrorDetails {
    pub fn new(
        pkg: String,
        file: String,
        line: u64,
        column: u64,
        log_id: Option<String>,
        msg: Option<String>,
    ) -> Self {
        Self {
            pkg,
            file,
            line,
            column,
            log_id,
            msg,
        }
    }
}

use fuel_tx::{ContractId, Receipt};

use crate::{
    codec::{ABIDecoder, DecoderConfig},
    traits::{Parameterize, Tokenizable},
    types::errors::{Error, Result, error},
};

#[derive(Clone)]
pub struct LogFormatter {
    formatter: fn(DecoderConfig, &[u8]) -> Result<String>,
    type_id: TypeId,
}

impl LogFormatter {
    pub fn new_log<T: Tokenizable + Parameterize + Debug + 'static>() -> Self {
        Self {
            formatter: Self::format_log::<T>,
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn new_error<T: Tokenizable + Parameterize + std::error::Error + 'static>() -> Self {
        Self {
            formatter: Self::format_error::<T>,
            type_id: TypeId::of::<T>(),
        }
    }

    fn format_log<T: Parameterize + Tokenizable + Debug>(
        decoder_config: DecoderConfig,
        bytes: &[u8],
    ) -> Result<String> {
        let token = ABIDecoder::new(decoder_config).decode(&T::param_type(), bytes)?;

        Ok(format!("{:?}", T::from_token(token)?))
    }

    fn format_error<T: Parameterize + Tokenizable + std::error::Error>(
        decoder_config: DecoderConfig,
        bytes: &[u8],
    ) -> Result<String> {
        let token = ABIDecoder::new(decoder_config).decode(&T::param_type(), bytes)?;

        Ok(T::from_token(token)?.to_string())
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
pub struct LogId(ContractId, String);

/// Struct used to pass the log mappings from the Abigen
#[derive(Debug, Clone, Default)]
pub struct LogDecoder {
    /// A mapping of LogId and param-type
    log_formatters: HashMap<LogId, LogFormatter>,
    error_codes: HashMap<u64, ErrorDetails>,
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
    pub fn new(
        log_formatters: HashMap<LogId, LogFormatter>,
        error_codes: HashMap<u64, ErrorDetails>,
    ) -> Self {
        Self {
            log_formatters,
            error_codes,
            decoder_config: Default::default(),
        }
    }

    pub fn get_error_codes(&self, id: &u64) -> Option<&ErrorDetails> {
        self.error_codes.get(id)
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
                    let token = ABIDecoder::new(self.decoder_config)
                        .decode(&T::param_type(), bytes.as_slice())?;

                    T::from_token(token)
                })
            })
            .collect()
    }

    /// Get LogIds and lazy decoders for specific type from a single receipt.
    pub fn decode_logs_lazy<'a, T: Tokenizable + Parameterize + 'static>(
        &'a self,
        receipt: &'a Receipt,
    ) -> impl Iterator<Item = impl FnOnce() -> Result<T>> + 'a {
        let target_ids: HashSet<&LogId> = self
            .log_formatters
            .iter()
            .filter(|(_, log_formatter)| log_formatter.can_handle_type::<T>())
            .map(|(log_id, _)| log_id)
            .collect();

        std::iter::once(receipt).extract_matching_logs_lazy::<T>(target_ids, self.decoder_config)
    }

    pub fn merge(&mut self, log_decoder: LogDecoder) {
        self.log_formatters.extend(log_decoder.log_formatters);
        self.error_codes.extend(log_decoder.error_codes);
    }
}

trait ExtractLogIdData {
    type Output: Iterator<Item = (LogId, Vec<u8>)>;
    fn extract_log_id_and_data(self) -> Self::Output;
}

trait ExtractLogIdLazy {
    fn extract_matching_logs_lazy<T: Tokenizable + Parameterize + 'static>(
        self,
        target_ids: HashSet<&LogId>,
        decoder_config: DecoderConfig,
    ) -> impl Iterator<Item = impl FnOnce() -> Result<T>>;
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
            } => Some((LogId(*id, (*rb).to_string()), data.clone())),
            Receipt::Log { ra, rb, id, .. } => {
                Some((LogId(*id, (*rb).to_string()), ra.to_be_bytes().to_vec()))
            }
            _ => None,
        })
    }
}

impl<'a, I: Iterator<Item = &'a Receipt>> ExtractLogIdLazy for I {
    fn extract_matching_logs_lazy<T: Tokenizable + Parameterize + 'static>(
        self,
        target_ids: HashSet<&LogId>,
        decoder_config: DecoderConfig,
    ) -> impl Iterator<Item = impl FnOnce() -> Result<T>> {
        self.filter_map(move |r| {
            let log_id = match r {
                Receipt::LogData { rb, id, .. } => LogId(*id, (*rb).to_string()),
                Receipt::Log { rb, id, .. } => LogId(*id, (*rb).to_string()),
                _ => return None,
            };

            if !target_ids.contains(&log_id) {
                return None;
            }

            enum Data<'a> {
                LogData(&'a [u8]),
                LogRa(u64),
            }

            let data = match r {
                Receipt::LogData {
                    data: Some(data), ..
                } => Some(Data::LogData(data.as_slice())),
                Receipt::Log { ra, .. } => Some(Data::LogRa(*ra)),
                _ => None,
            };

            data.map(move |data| {
                move || {
                    let normalized_data = match data {
                        Data::LogData(data) => data,
                        Data::LogRa(ra) => &ra.to_be_bytes(),
                    };
                    let token = ABIDecoder::new(decoder_config)
                        .decode(&T::param_type(), normalized_data)?;
                    T::from_token(token)
                }
            })
        })
    }
}

pub fn log_formatters_lookup(
    log_id_log_formatter_pairs: Vec<(String, LogFormatter)>,
    contract_id: ContractId,
) -> HashMap<LogId, LogFormatter> {
    log_id_log_formatter_pairs
        .into_iter()
        .map(|(id, log_formatter)| (LogId(contract_id, id), log_formatter))
        .collect()
}
