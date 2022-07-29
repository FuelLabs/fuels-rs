#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::{Config, FuelService};

#[cfg(feature = "fuel-core-lib")]
use fuels_signers::provider::Provider;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::launch_provider_and_get_wallet;

use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuels_contract::script::Script;
use fuels_types::errors::Error;

/// Run the Sway script binary located at `binary_filepath` and return its resulting receipts,
/// without having to setup a node or contract bindings.
#[allow(dead_code)]
#[cfg(feature = "fuel-core-lib")]
pub async fn run_compiled_script(binary_filepath: &str) -> Result<Vec<Receipt>, Error> {
    let script_binary = std::fs::read(binary_filepath)?;
    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let provider = Provider::connect(server.bound_address).await?;

    let script = get_script(script_binary);

    script.call(&provider).await
}

#[allow(dead_code)]
#[cfg(not(feature = "fuel-core-lib"))]
pub async fn run_compiled_script(binary_filepath: &str) -> Result<Vec<Receipt>, Error> {
    let script_binary = std::fs::read(binary_filepath)?;

    let wallet = launch_provider_and_get_wallet().await;
    let provider = wallet.get_provider()?;

    let script = get_script(script_binary);

    script.call(provider).await
}

fn get_script(script_binary: Vec<u8>) -> Script {
    let tx = Transaction::Script {
        gas_price: 0,
        gas_limit: 1000000,
        maturity: 0,
        byte_price: 0,
        receipts_root: Default::default(),
        script: script_binary, // Pass the compiled script into the tx
        script_data: vec![],
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![vec![].into()],
        metadata: None,
    };

    Script::new(tx)
}

#[cfg(test)]
mod tests {
    use crate::script::run_compiled_script;
    use fuels_types::errors::Error;

    #[tokio::test]
    async fn test_run_compiled_script() -> Result<(), Error> {
        // ANCHOR: run_compiled_script
        let path_to_bin = "../fuels/tests/test_projects/logging/out/debug/logging.bin";
        let return_val = run_compiled_script(path_to_bin).await?;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a");

        assert_eq!(correct_hex?, return_val[0].data().unwrap());
        // ANCHOR_END: run_compiled_script
        Ok(())
    }
}
