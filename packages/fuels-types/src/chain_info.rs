#![cfg(feature = "std")]

use crate::consensus_parameters::ConsensusParameters;
use fuel_core_client::client::schema::chain::ChainInfo as ClientChainInfo;
use fuel_tx::ConsensusParameters as TxConsensusParameters;

use crate::block::Block;

#[derive(Debug)]
pub struct ChainInfo {
    pub base_chain_height: u32,
    pub name: String,
    pub peer_count: i32,
    pub latest_block: Block,
    pub consensus_parameters: ConsensusParameters,
}

impl From<ClientChainInfo> for ChainInfo {
    fn from(client_chain_info: ClientChainInfo) -> Self {
        let consensus_params_tx: TxConsensusParameters =
            client_chain_info.consensus_parameters.into();
        Self {
            base_chain_height: client_chain_info.base_chain_height.0.into(),
            name: client_chain_info.name,
            peer_count: client_chain_info.peer_count,
            latest_block: client_chain_info.latest_block.into(),
            consensus_parameters: consensus_params_tx.into(),
        }
    }
}
