use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;
use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuels_contract::script::Script;
use std::fs::read;

//`run_script` is helper function for testing simple Sway scripts and reducing boilerplate code related to setting up contracts and deployment.
pub async fn run_script(bin_path: &str) -> Vec<Receipt> {
    let bin = read(bin_path);
    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let client = FuelClient::from(server.bound_address);

    let tx = Transaction::Script {
        gas_price: 0,
        gas_limit: 1000000,
        maturity: 0,
        byte_price: 0,
        receipts_root: Default::default(),
        script: bin.unwrap(), // Here we pass the compiled script into the transaction
        script_data: vec![],
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![vec![].into()],
        metadata: None,
    };

    let script = Script::new(tx);
    let receipts = script.call(&client).await.unwrap();

    receipts
}

#[cfg(test)]
mod tests {

    use crate::run_script;

    #[tokio::test]
    // ANCHOR: test_logging_sway
    async fn test_logging_sway() {
        let path_to_bin = "../fuels-abigen-macro/tests/test_projects/logging/out/debug/logging.bin";
        let return_val = run_script(path_to_bin).await;

        let correct_hex =
            hex::decode("ef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a");

        assert_eq!(correct_hex.unwrap(), return_val[0].data().unwrap());
    }
    // ANCHOR_END: test_logging_sway
}
