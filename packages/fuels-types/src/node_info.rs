#![cfg(feature = "std")]

use fuel_core_client::client::schema::node_info::NodeInfo as ClientNodeInfo;

#[derive(Debug)]
pub struct NodeInfo {
    pub utxo_validation: bool,
    pub vm_backtrace: bool,
    pub min_gas_price: u64,
    pub max_tx: u64,
    pub max_depth: u64,
    pub node_version: String,
}

impl From<ClientNodeInfo> for NodeInfo {
    fn from(client_node_info: ClientNodeInfo) -> Self {
        Self {
            utxo_validation: client_node_info.utxo_validation,
            vm_backtrace: client_node_info.vm_backtrace,
            min_gas_price: client_node_info.min_gas_price.0,
            max_tx: client_node_info.max_tx.0,
            max_depth: client_node_info.max_depth.0,
            node_version: client_node_info.node_version,
        }
    }
}
