use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;
use fuel_gql_client::fuel_tx::{Receipt, Transaction};
use fuel_tx::consts::MAX_GAS_PER_TX;
use fuels_contract::script::Script;
use std::fs::read;

pub async fn script_runner(bin_path: &str) -> Vec<Receipt> {
    let bin = read(bin_path);
    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let client = FuelClient::from(server.bound_address);

    let tx = Transaction::Script {
        gas_price: 0,
        gas_limit: MAX_GAS_PER_TX,
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
