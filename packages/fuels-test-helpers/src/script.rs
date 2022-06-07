use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;
use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuels_contract::script::Script;
use fuels_core::errors::Error;

/// Run the Sway script binary located at `binary_filepath` and return its resulting receipts,
/// without having to setup a node or contract bindings.
pub async fn run_compiled_script(binary_filepath: &str) -> Result<Vec<Receipt>, Error> {
    let script_binary = std::fs::read(binary_filepath)?;
    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let client = FuelClient::from(server.bound_address);

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

    let script = Script::new(tx);
    script.call(&client).await
}

#[cfg(test)]
mod tests {
    use crate::run_compiled_script;

    #[tokio::test]
    // ANCHOR: run_compiled_script
    async fn test_run_compiled_script() {
        let path_to_bin = "../fuels-abigen-macro/tests/test_projects/logging/out/debug/logging.bin";
        let return_val = run_compiled_script(path_to_bin).await;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a");

        assert_eq!(correct_hex.unwrap(), return_val.unwrap()[0].data().unwrap());
    }
    // ANCHOR_END: run_compiled_script
}
