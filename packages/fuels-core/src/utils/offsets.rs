use crate::{error, types::errors::Result};
use fuel_asm::Instruction;
use fuel_tx::{
    field::{Script, StorageSlots},
    ConsensusParameters, InputRepr,
};
use fuel_types::{bytes::padded_len_usize, ContractId};

/// Gets the base offset for a script or a predicate. The offset depends on the `max_inputs`
/// field of the `ConsensusParameters` and the static offset.
pub fn base_offset_script(consensus_parameters: &ConsensusParameters) -> usize {
    consensus_parameters.tx_params().tx_offset() + fuel_tx::Script::script_offset_static()
}

/// Gets the base offset for a script or a predicate. The offset depends on the `max_inputs`
/// field of the `ConsensusParameters` and the static offset.
pub fn base_offset_create(consensus_parameters: &ConsensusParameters) -> usize {
    // The easiest way to get the offset of `fuel_tx::Create` is to get the offset of the last field
    // of `Create` -- i.e. `salt` and skip it by adding its length.
    // This should be updated if `fuel_tx::Create` ever adds more fields after `salt`.
    consensus_parameters.tx_params().tx_offset() + fuel_tx::Create::storage_slots_offset_static()
}

/// Calculates the length of the script based on the number of contract calls it
/// has to make and returns the offset at which the script data begins
pub fn call_script_data_offset(
    consensus_parameters: &ConsensusParameters,
    calls_instructions_len: usize,
) -> Result<usize> {
    // Instruction::SIZE is a placeholder for the RET instruction which is added later for returning
    // from the script. This doesn't happen in the predicate.
    let opcode_len = Instruction::SIZE;

    let padded_len = padded_len_usize(calls_instructions_len + opcode_len).ok_or_else(|| {
        error!(
            Other,
            "call script data len overflow: {calls_instructions_len}"
        )
    })?;
    Ok(base_offset_script(consensus_parameters) + padded_len)
}

pub fn coin_predicate_data_offset(code_len: usize) -> Result<usize> {
    let code_len_padded = padded_len_usize(code_len)
        .ok_or_else(|| error!(Other, "coin predicate code len overflow: {code_len}"))?;

    Ok(InputRepr::Coin
        .coin_predicate_offset()
        .expect("should have predicate offset")
        + code_len_padded)
}

pub fn message_predicate_data_offset(message_data_len: usize, code_len: usize) -> Result<usize> {
    let data_len_padded = padded_len_usize(message_data_len).ok_or_else(|| {
        error!(
            Other,
            "message predicate data len overflow: {message_data_len}"
        )
    })?;
    let code_len_padded = padded_len_usize(code_len)
        .ok_or_else(|| error!(Other, "message predicate code len overflow: {code_len}"))?;

    Ok(InputRepr::Message
        .data_offset()
        .expect("should have data offset")
        + data_len_padded
        + code_len_padded)
}

pub fn coin_signed_data_offset() -> usize {
    InputRepr::Coin
        .coin_predicate_offset()
        .expect("should have coin offset")
}

pub fn message_signed_data_offset(message_data_len: usize) -> Result<usize> {
    let padded_len = padded_len_usize(message_data_len).ok_or_else(|| {
        error!(
            Other,
            "signed message data len overflow: {message_data_len}"
        )
    })?;

    Ok(InputRepr::Message
        .data_offset()
        .expect("should have data offset")
        + padded_len)
}

pub fn contract_input_offset() -> usize {
    // The easiest way to get the contract input offset is to get the offset of the last field of
    // `InputRepr::Contract` -- i.e. the `contract_id` and then add its len to skip the last field.
    // Care should be taken to update this should `InputRepr::Contract` ever get another field after
    // this last one.
    InputRepr::Contract.contract_id_offset().unwrap() + ContractId::LEN
}
