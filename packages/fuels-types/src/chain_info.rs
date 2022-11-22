use fuel_gql_client::client::schema::chain::{
    ChainInfo as SchemaChainInfo, ConsensusParameters as SchemaConsensusParams,
};

use crate::block::Block;

#[derive(Debug)]
pub struct ChainInfo {
    schema_chain_info: SchemaChainInfo,
}

impl From<SchemaChainInfo> for ChainInfo {
    fn from(schema_chain_info: SchemaChainInfo) -> Self {
        Self { schema_chain_info }
    }
}

impl ChainInfo {
    pub fn base_chain_height(&self) -> u64 {
        self.schema_chain_info.base_chain_height.0
    }

    pub fn name(&self) -> &str {
        &self.schema_chain_info.name
    }

    pub fn peer_count(&self) -> i32 {
        self.schema_chain_info.peer_count
    }

    pub fn latest_block(&self) -> Block {
        Block {
            schema_block: self.schema_chain_info.latest_block.clone(),
        }
    }

    pub fn consensus_parameters(&self) -> ConsensusParameters {
        ConsensusParameters {
            schema_consensus_params: &self.schema_chain_info.consensus_parameters,
        }
    }
}

#[derive(Debug)]
pub struct ConsensusParameters<'a> {
    schema_consensus_params: &'a SchemaConsensusParams,
}

impl<'a> ConsensusParameters<'a> {
    pub fn contract_max_size(&self) -> u64 {
        self.schema_consensus_params.contract_max_size.0
    }

    pub fn max_inputs(&self) -> u64 {
        self.schema_consensus_params.max_inputs.0
    }

    pub fn max_outputs(&self) -> u64 {
        self.schema_consensus_params.max_outputs.0
    }

    pub fn max_witnesses(&self) -> u64 {
        self.schema_consensus_params.max_witnesses.0
    }

    pub fn max_gas_per_tx(&self) -> u64 {
        self.schema_consensus_params.max_gas_per_tx.0
    }

    pub fn max_script_length(&self) -> u64 {
        self.schema_consensus_params.max_script_length.0
    }

    pub fn max_script_data_length(&self) -> u64 {
        self.schema_consensus_params.max_script_data_length.0
    }

    pub fn max_storage_slots(&self) -> u64 {
        self.schema_consensus_params.max_storage_slots.0
    }

    pub fn max_predicate_length(&self) -> u64 {
        self.schema_consensus_params.max_predicate_length.0
    }

    pub fn max_predicate_data_length(&self) -> u64 {
        self.schema_consensus_params.max_predicate_data_length.0
    }

    pub fn gas_price_factor(&self) -> u64 {
        self.schema_consensus_params.gas_price_factor.0
    }

    pub fn gas_per_byte(&self) -> u64 {
        self.schema_consensus_params.gas_per_byte.0
    }

    pub fn max_message_data_length(&self) -> u64 {
        self.schema_consensus_params.max_message_data_length.0
    }
}
