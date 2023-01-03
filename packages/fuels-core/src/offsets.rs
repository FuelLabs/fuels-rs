use crate::tx::{field::Script, ConsensusParameters, InputRepr};
use fuel_types::bytes::padded_len_usize;
use fuel_vm::prelude::Opcode;

/// Gets the base offset for a script or a predicate. The offset depends on the `max_inputs`
/// field of the `ConsensusParameters` and the static offset.
pub fn base_offset(consensus_parameters: &ConsensusParameters) -> usize {
    consensus_parameters.tx_offset() + fuel_tx::Script::script_offset_static()
}

/// Calculates the length of the script based on the number of contract calls it
/// has to make and returns the offset at which the script data begins
pub fn call_script_data_offset(
    consensus_parameters: &ConsensusParameters,
    calls_instructions_len: usize,
) -> usize {
    // Opcode::LEN is a placeholder for the RET instruction which is added later for returning
    // from the script. This doesn't happen in the predicate.
    let opcode_len = Opcode::LEN;

    base_offset(consensus_parameters) + padded_len_usize(calls_instructions_len + opcode_len)
}

pub fn coin_predicate_data_offset(code_len: usize) -> usize {
    InputRepr::Coin
        .coin_predicate_offset()
        .expect("should have predicate offset")
        + padded_len_usize(code_len)
}

pub fn message_predicate_data_offset(message_data_len: usize, code_len: usize) -> usize {
    InputRepr::Message
        .data_offset()
        .expect("should have data offset")
        + padded_len_usize(message_data_len)
        + padded_len_usize(code_len)
}
