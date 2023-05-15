use crate::consensus_parameters::ConsensusParameters;
use fuel_core_chain_config::ChainConfig as ClientChainConfig;
use fuel_vm::prelude::GasCosts;

use fuel_core_chain_config::{ConsensusConfig, StateConfig};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainConfig {
    pub chain_name: String,
    pub block_gas_limit: u64,
    pub initial_state: Option<StateConfig>,
    pub consensus_parameters: ConsensusParameters,
    pub gas_costs: GasCosts,
    pub consensus: ConsensusConfig,
}

impl From<ClientChainConfig> for ChainConfig {
    fn from(client_chain_config: ClientChainConfig) -> Self {
        Self {
            chain_name: client_chain_config.chain_name,
            block_gas_limit: client_chain_config.block_gas_limit,
            initial_state: client_chain_config.initial_state,
            consensus_parameters: client_chain_config.transaction_parameters.into(),
            gas_costs: client_chain_config.gas_costs,
            consensus: client_chain_config.consensus,
        }
    }
}
impl Default for ChainConfig {
    fn default() -> Self {
        ClientChainConfig::default().into()
    }
}

impl ChainConfig {
    fn with_consensus_parameters(mut self, consensus_parameters: ConsensusParameters) -> Self {
        self.consensus_parameters = consensus_parameters;
        self
    }
}
