use crate::errors::Error;
use forc::test::{forc_build, BuildCommand};
use forc::util::constants;
use forc::util::helpers::{find_manifest_dir, read_manifest};
use fuel_client::client::FuelClient;
use fuel_tx::{Input, Output, Receipt, Transaction};
use std::path::PathBuf;

/// Script is a very thin layer on top of fuel-client with some
/// extra functionalities needed and provided by the SDK.
pub struct Script {
    pub tx: Transaction,
}

#[derive(Debug, Clone)]
pub struct CompiledScript {
    pub raw: Vec<u8>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub target_network_url: String,
}

impl Script {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    pub async fn call(self, fuel_client: &FuelClient) -> Result<Vec<Receipt>, Error> {
        let tx_id = fuel_client.submit(&self.tx).await.unwrap();

        let receipts = fuel_client.receipts(&tx_id.0.to_string()).await.unwrap();

        Ok(receipts)
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
        };

        let raw =
            forc_build::build(build_command).map_err(|message| Error::CompilationError(message))?;

        let manifest_dir = find_manifest_dir(&PathBuf::from(project_path)).unwrap();
        let manifest = read_manifest(&manifest_dir).map_err(|e| {
            Error::CompilationError(format!("Failed to find manifest for contract: {}", e))
        })?;

        let (inputs, outputs) = manifest.get_tx_inputs_and_outputs().map_err(|e| {
            Error::CompilationError(format!(
                "Failed to find contract's inputs and outputs: {}",
                e
            ))
        })?;

        let node_url = match &manifest.network {
            Some(network) => &network.url,
            _ => constants::DEFAULT_NODE_URL,
        };

        Ok(CompiledScript {
            raw,
            inputs,
            outputs,
            target_network_url: node_url.to_string(),
        })
    }
}
