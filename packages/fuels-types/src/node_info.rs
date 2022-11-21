use fuel_gql_client::client::schema::node_info::NodeInfo as SchemaNodeInfo;

#[derive(Debug)]
pub struct NodeInfo {
    schema_node_info: SchemaNodeInfo,
}

impl From<SchemaNodeInfo> for NodeInfo {
    fn from(schema_node_info: SchemaNodeInfo) -> Self {
        Self { schema_node_info }
    }
}

impl NodeInfo {
    pub fn utxo_validation(&self) -> bool {
        self.schema_node_info.utxo_validation
    }

    pub fn vm_backtrace(&self) -> bool {
        self.schema_node_info.vm_backtrace
    }

    pub fn min_gas_price(&self) -> u64 {
        self.schema_node_info.min_gas_price.0
    }

    pub fn max_tx(&self) -> u64 {
        self.schema_node_info.max_tx.0
    }

    pub fn max_depth(&self) -> u64 {
        self.schema_node_info.max_depth.0
    }

    pub fn node_version(&self) -> &str {
        &self.schema_node_info.node_version
    }
}
