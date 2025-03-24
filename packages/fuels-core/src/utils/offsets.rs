use fuel_asm::Instruction;
use fuel_tx::{ConsensusParameters, field::Script};
use fuel_types::bytes::padded_len_usize;

use crate::{error, types::errors::Result};

/// Gets the base offset for a script or a predicate. The offset depends on the `max_inputs`
/// field of the `ConsensusParameters` and the static offset.
pub fn base_offset_script(consensus_parameters: &ConsensusParameters) -> usize {
    consensus_parameters.tx_params().tx_offset() + fuel_tx::Script::script_offset_static()
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
