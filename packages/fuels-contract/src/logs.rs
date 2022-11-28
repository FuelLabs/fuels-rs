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
            .map(|((c_id, log_id), data)| {
                let param_type = self
                    .logs_map
                    .get(&(c_id, log_id))
                    .ok_or_else(|| Error::InvalidData("Failed to find log id".into()))?;

                param_type.decode_log(&data)
            })
            .collect::<Result<Vec<String>, Error>>()
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

        let decoded_logs: Vec<T> = receipts
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
            .collect::<Result<Vec<_>, _>>()?;

        Ok(decoded_logs)
    }

    pub fn merge(&mut self, log_decoder: &LogDecoder) {
        self.logs_map
            .extend(log_decoder.logs_map.clone().into_iter());
    }
}
