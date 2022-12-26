use fuel_gql_client::fuel_tx::Receipt;
use fuels_core::{try_from_bytes, DecodableLog, Parameterize, Tokenizable};
use fuels_types::{bech32::Bech32ContractId, errors::Error, param_types::ParamType};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

/// Struct used to pass the log mappings from the Abigen
#[derive(Debug, Clone, Default)]
pub struct LogDecoder {
    /// A mapping of (contract-id, log-id) and param-type
    pub logs_map: HashMap<(Bech32ContractId, u64), ParamType>,
}

impl LogDecoder {
    /// Get all decoded logs from the given receipts as `String`
    pub fn get_logs(&self, receipts: &[Receipt]) -> Result<Vec<String>, Error> {
        let ids_with_data = receipts.iter().filter_map(|r| match r {
            Receipt::LogData { rb, data, id, .. } => {
                Some(((Bech32ContractId::from(*id), *rb), data.clone()))
            }
            Receipt::Log { ra, rb, id, .. } => Some((
                (Bech32ContractId::from(*id), *rb),
                ra.to_be_bytes().to_vec(),
            )),
            _ => None,
        });

        ids_with_data
            .filter_map(|((c_id, log_id), data)| {
                self.logs_map
                    .get(&(c_id, log_id))
                    .map(|param_type| (param_type, data))
            })
            .map(|(param_type, data)| param_type.decode_log(&data))
            .collect()
    }

    /// Get decoded logs with specific type from the given receipts.
    /// Note that this method returns the actual type and not a `String` representation.
    pub fn get_logs_with_type<T: Tokenizable + Parameterize>(
        &self,
        receipts: &[Receipt],
    ) -> Result<Vec<T>, Error> {
        let target_param_type = T::param_type();

        let target_ids: HashSet<(Bech32ContractId, u64)> = self
            .logs_map
            .iter()
            .filter_map(|((c_id, log_id), param_type)| {
                if *param_type == target_param_type {
                    Some((c_id.clone(), *log_id))
                } else {
                    None
                }
            })
            .collect();

        receipts
            .iter()
            .filter_map(|r| match r {
                Receipt::LogData { id, rb, data, .. }
                    if target_ids.contains(&(Bech32ContractId::from(*id), *rb)) =>
                {
                    Some(data.clone())
                }
                Receipt::Log { id, ra, rb, .. }
                    if target_ids.contains(&(Bech32ContractId::from(*id), *rb)) =>
                {
                    Some(ra.to_be_bytes().to_vec())
                }
                _ => None,
            })
            .map(|data| try_from_bytes(&data))
            .collect()
    }

    pub fn merge(&mut self, log_decoder: &LogDecoder) {
        self.logs_map
            .extend(log_decoder.logs_map.clone().into_iter());
    }
}

/// Decodes the logged type from the receipt of a `RevertTransactionError` if available
pub fn decode_revert_error(err: Error, log_decoder: &LogDecoder) -> Error {
    if let Error::RevertTransactionError(_, receipts) = &err {
        if let Ok(logs) = log_decoder.get_logs(receipts) {
            if let Some(log) = logs.last() {
                return Error::RevertTransactionError(log.to_string(), receipts.to_owned());
            }
        }
    }
    err
}
