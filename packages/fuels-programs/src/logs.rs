use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    iter::FilterMap,
};

use fuel_tx::Receipt;
use fuels_core::{traits::DecodableLog, try_from_bytes};
use fuels_types::{
    bech32::Bech32ContractId,
    errors::{Error, Result},
    param_types::ParamType,
    traits::{Parameterize, Tokenizable},
};

const REQUIRE_ID: u64 = 0xffff_ffff_ffff_0000;
const ASSERT_EQ_ID: u64 = 0xffff_ffff_ffff_0003;

/// Struct used to pass the log mappings from the Abigen
#[derive(Debug, Clone, Default)]
pub struct LogDecoder {
    /// A mapping of (contract-id, log-id) and param-type
    pub type_lookup: HashMap<(Bech32ContractId, u64), ParamType>,
}

impl LogDecoder {
    /// Get all decoded logs from the given receipts as `String`
    pub fn get_logs(&self, receipts: &[Receipt]) -> Result<Vec<String>> {
        receipts
            .iter()
            .filter_map_log_receipts()
            .filter_map(|((c_id, log_id), data)| {
                self.type_lookup
                    .get(&(c_id, log_id))
                    .map(|param_type| param_type.decode_log(&data))
            })
            .collect()
    }

    /// Get decoded logs with specific type from the given receipts.
    /// Note that this method returns the actual type and not a `String` representation.
    pub fn get_logs_with_type<T: Tokenizable + Parameterize>(
        &self,
        receipts: &[Receipt],
    ) -> Result<Vec<T>> {
        let target_param_type = T::param_type();

        let target_ids: HashSet<(Bech32ContractId, u64)> = self
            .type_lookup
            .iter()
            .filter_map(|(log_id, param_type)| {
                (*param_type == target_param_type).then_some(log_id.clone())
            })
            .collect();

        receipts
            .iter()
            .filter_map_log_receipts()
            .filter_map(|(log_id, data)| {
                target_ids
                    .contains(&log_id)
                    .then_some(try_from_bytes(&data))
            })
            .collect()
    }

    pub fn merge(&mut self, log_decoder: LogDecoder) {
        self.type_lookup.extend(log_decoder.type_lookup.into_iter());
    }
}

trait FilterMapLogReceipts {
    type Output: Iterator<Item = ((Bech32ContractId, u64), Vec<u8>)>;
    fn filter_map_log_receipts(self) -> Self::Output;
}

impl<'a, I: Iterator<Item = &'a Receipt>> FilterMapLogReceipts for I {
    type Output = FilterMap<Self, fn(&Receipt) -> Option<((Bech32ContractId, u64), Vec<u8>)>>;
    fn filter_map_log_receipts(self) -> Self::Output {
        self.filter_map(|r| match r {
            Receipt::LogData { rb, data, id, .. } => {
                Some(((Bech32ContractId::from(*id), *rb), data.clone()))
            }
            Receipt::Log { ra, rb, id, .. } => Some((
                (Bech32ContractId::from(*id), *rb),
                ra.to_be_bytes().to_vec(),
            )),
            _ => None,
        })
    }
}

/// Decodes the logged type from the receipt of a `RevertTransactionError` if available
pub fn decode_revert_error(err: Error, log_decoder: &LogDecoder) -> Error {
    if let Error::RevertTransactionError {
        revert_id,
        receipts,
        ..
    } = &err
    {
        match *revert_id {
            REQUIRE_ID => return decode_require_revert(log_decoder, receipts),
            ASSERT_EQ_ID => return decode_assert_eq_revert(log_decoder, receipts),
            _ => {}
        }
    }
    err
}

fn decode_require_revert(log_decoder: &LogDecoder, receipts: &[Receipt]) -> Error {
    let reason = log_decoder
        .get_logs(receipts)
        .ok()
        .and_then(|logs| logs.last().cloned())
        .unwrap_or_else(|| "Filed to decode log from require revert".to_string());

    Error::RevertTransactionError {
        reason,
        revert_id: REQUIRE_ID,
        receipts: receipts.to_owned(),
    }
}

fn decode_assert_eq_revert(log_decoder: &LogDecoder, receipts: &[Receipt]) -> Error {
    let reason = log_decoder
        .get_logs(receipts)
        .ok()
        .and_then(|logs| {
            if let [.., lhs, rhs] = logs.as_slice() {
                return Some(format!(
                    "assertion filed: `(left == right)`\n left: `{lhs:?}`\n right: `{rhs:?}`"
                ));
            }
            None
        })
        .unwrap_or_else(|| "Filed to decode logs from assert_eq revert".to_string());

    Error::RevertTransactionError {
        reason,
        revert_id: ASSERT_EQ_ID,
        receipts: receipts.to_owned(),
    }
}
