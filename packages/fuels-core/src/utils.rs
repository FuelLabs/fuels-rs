use std::collections::HashMap;

use fuels_types::{bech32::Bech32ContractId, param_types::ParamType, ByteArray, ContractId};
use sha2::{Digest, Sha256};

/// Hashes an encoded function selector using SHA256 and returns the first 4 bytes.
/// The function selector has to have been already encoded following the ABI specs defined
/// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
pub fn first_four_bytes_of_sha256_hash(string: &str) -> ByteArray {
    let string_as_bytes = string.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(string_as_bytes);
    let result = hasher.finalize();
    let mut output = ByteArray::default();
    output[4..].copy_from_slice(&result[..4]);
    output
}

pub fn log_type_lookup(
    id_param_pairs: &[(u64, ParamType)],
    contract_id: Option<Bech32ContractId>,
) -> HashMap<(Bech32ContractId, u64), ParamType> {
    let contract_id = contract_id.unwrap_or_else(|| Bech32ContractId::from(ContractId::zeroed()));
    id_param_pairs
        .iter()
        .map(|(id, param_type)| ((contract_id.clone(), *id), param_type.to_owned()))
        .collect()
}
