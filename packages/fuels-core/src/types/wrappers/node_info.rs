#![cfg(feature = "std")]

use fuel_core_client::client::types::node_info::NodeInfo as ClientNodeInfo;

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub utxo_validation: bool,
    pub vm_backtrace: bool,
    pub max_tx: u64,
    pub max_depth: u64,
    pub node_version: String,
}

impl From<ClientNodeInfo> for NodeInfo {
    fn from(client_node_info: ClientNodeInfo) -> Self {
        Self {
            utxo_validation: client_node_info.utxo_validation,
            vm_backtrace: client_node_info.vm_backtrace,
            max_tx: client_node_info.max_tx,
            max_depth: client_node_info.max_depth,
            node_version: client_node_info.node_version,
        }
    }
}
