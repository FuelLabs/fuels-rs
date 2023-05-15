use crate::constants::WORD_SIZE;
use fuel_tx::ConsensusParameters as TxConsensusParameters;
use fuel_types::{AssetId, Bytes32};
use std::cmp::max;

/// Consensus configurable parameters used for verifying transactions
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConsensusParameters {
    /// Maximum contract size, in bytes.
    pub contract_max_size: u64,
    /// Maximum number of inputs.
    pub max_inputs: u64,
    /// Maximum number of outputs.
    pub max_outputs: u64,
    /// Maximum number of witnesses.
    pub max_witnesses: u64,
    /// Maximum gas per transaction.
    pub max_gas_per_tx: u64,
    /// Maximum length of script, in instructions.
    pub max_script_length: u64,
    /// Maximum length of script data, in bytes.
    pub max_script_data_length: u64,
    /// Maximum number of initial storage slots.
    pub max_storage_slots: u64,
    /// Maximum length of predicate, in instructions.
    pub max_predicate_length: u64,
    /// Maximum length of predicate data, in bytes.
    pub max_predicate_data_length: u64,
    /// Factor to convert between gas and transaction assets value.
    pub gas_price_factor: u64,
    /// A fixed ratio linking metered bytes to gas price
    pub gas_per_byte: u64,
    /// Maximum length of message data, in bytes.
    pub max_message_data_length: u64,
    /// The unique identifier of this chain
    pub chain_id: u64,
}

impl Default for ConsensusParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ConsensusParameters {
    /// Transaction memory offset in VM runtime
    pub const fn tx_offset(&self) -> usize {
        Bytes32::LEN // Tx ID
            + WORD_SIZE // Tx size
            // Asset ID/Balance coin input pairs
            + self.max_inputs as usize * (AssetId::LEN + WORD_SIZE)
    }

    /// Default consensus parameters with settings suggested in fuel-specs
    pub const DEFAULT: Self = Self {
        contract_max_size: 16 * 1024 * 1024,
        max_inputs: 255,
        max_outputs: 255,
        max_witnesses: 255,
        max_gas_per_tx: 100_000_000,
        max_script_length: 1024 * 1024,
        max_script_data_length: 1024 * 1024,
        max_storage_slots: 255,
        max_predicate_length: 1024 * 1024,
        max_predicate_data_length: 1024 * 1024,
        gas_price_factor: 1_000_000_000,
        gas_per_byte: 4,
        max_message_data_length: 1024 * 1024,
        chain_id: 0,
    };

    pub const fn with_max_inputs(self, max_inputs: u64) -> Self {
        Self { max_inputs, ..self }
    }
}

impl From<TxConsensusParameters> for ConsensusParameters {
    fn from(tx_consensus_parameters: TxConsensusParameters) -> Self {
        Self {
            contract_max_size: tx_consensus_parameters.contract_max_size,
            max_inputs: tx_consensus_parameters.max_inputs,
            max_outputs: tx_consensus_parameters.max_outputs,
            max_witnesses: tx_consensus_parameters.max_witnesses,
            max_gas_per_tx: tx_consensus_parameters.max_gas_per_tx,
            max_script_length: tx_consensus_parameters.max_gas_per_tx,
            max_script_data_length: tx_consensus_parameters.max_script_data_length,
            max_storage_slots: tx_consensus_parameters.max_storage_slots,
            max_predicate_length: tx_consensus_parameters.max_predicate_length,
            max_predicate_data_length: tx_consensus_parameters.max_predicate_data_length,
            gas_price_factor: tx_consensus_parameters.gas_price_factor,
            gas_per_byte: tx_consensus_parameters.gas_per_byte,
            max_message_data_length: tx_consensus_parameters.max_message_data_length,
            chain_id: tx_consensus_parameters.chain_id,
        }
    }
}

impl From<ConsensusParameters> for TxConsensusParameters {
    fn from(params: ConsensusParameters) -> Self {
        Self {
            contract_max_size: params.contract_max_size,
            max_inputs: params.max_inputs,
            max_outputs: params.max_outputs,
            max_witnesses: params.max_witnesses,
            max_gas_per_tx: params.max_gas_per_tx,
            max_script_length: params.max_gas_per_tx,
            max_script_data_length: params.max_script_data_length,
            max_storage_slots: params.max_storage_slots,
            max_predicate_length: params.max_predicate_length,
            max_predicate_data_length: params.max_predicate_data_length,
            gas_price_factor: params.gas_price_factor,
            gas_per_byte: params.gas_per_byte,
            max_message_data_length: params.max_message_data_length,
            chain_id: params.chain_id,
        }
    }
}
