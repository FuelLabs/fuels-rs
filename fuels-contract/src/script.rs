use crate::errors::Error;
use forc::test::{forc_build, BuildCommand};
use forc::util::constants;
use forc::util::helpers::read_manifest;
use fuel_gql_client::client::{types::TransactionStatus, FuelClient};
use fuel_tx::{Receipt, Transaction};
use std::path::PathBuf;
use sway_utils::find_manifest_dir;

/// Script is a very thin layer on top of fuel-client with some
/// extra functionalities needed and provided by the SDK.
pub struct Script {
    pub tx: Transaction,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub raw: Vec<u8>,
    pub target_network_url: String,
}

impl Script {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await.unwrap().0.to_string();

        let receipts = fuel_client.receipts(&tx_id).await?;
        let status = fuel_client.transaction_status(&tx_id).await?;
        match status {
            TransactionStatus::Failure { reason, .. } => Err(Error::ContractCallError(reason)),
            _ => Ok(receipts),
        }
    }

    /// Compiles a Sway script
    pub fn compile_sway_script(project_path: &str) -> Result<CompiledScript, Error> {
        let build_command = BuildCommand {
            path: Some(project_path.into()),
            print_finalized_asm: false,
            print_intermediate_asm: false,
            binary_outfile: None,
            offline_mode: false,
            silent_mode: true,
            use_ir: false,
            print_ir: false,
            debug_outfile: None,
            minify_json_abi: false,
            output_directory: None,
        };

        let (raw, _) = forc_build::build(build_command).map_err(Error::CompilationError)?;

        let manifest_dir = find_manifest_dir(&PathBuf::from(project_path)).unwrap();
        let manifest = read_manifest(&manifest_dir).map_err(|e| {
            Error::CompilationError(format!("Failed to find manifest for contract: {}", e))
        })?;

        let node_url = match &manifest.network {
            Some(network) => &network.url,
            _ => constants::DEFAULT_NODE_URL,
        };

        Ok(CompiledScript {
            raw,
            target_network_url: node_url.to_string(),
        })
    }
}
