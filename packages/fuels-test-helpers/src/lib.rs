//! Testing helpers/utilities for Fuel SDK.

pub use fuel_core::service::Config;
use fuel_core::{
    chain_config::{ChainConfig, CoinConfig, StateConfig},
    model::CoinStatus,
    service::{DbType, FuelService},
};
use std::env;

use fuel_core_interfaces::model::Coin; // TODO Emir make this optional

use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Address, Bytes32, UtxoId},
};
use rand::Fill;
use serde_json::Value;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;
use tokio::process::{Child, Command};

// #[cfg(feature = "fuels-signers")]
mod signers;
mod wallets_config;

mod node_config_json; // Todo Emir make this optional

use crate::node_config_json::{get_node_config_json, DummyConfig}; // Todo make optional
#[cfg(feature = "fuels-signers")]
pub use signers::*;
pub use wallets_config::*;

pub fn setup_coins(owner: Address, num_coins: u64, amount: u64) -> Vec<(UtxoId, Coin)> {
    let mut rng = rand::thread_rng();

    let coins: Vec<(UtxoId, Coin)> = (1..=num_coins)
        .map(|_i| {
            let coin = Coin {
                owner,
                amount,
                asset_id: Default::default(),
                maturity: Default::default(),
                status: CoinStatus::Unspent,
                block_created: Default::default(),
            };

            let mut r = Bytes32::zeroed();
            r.try_fill(&mut rng).unwrap();
            let utxo_id = UtxoId::new(r, 0);
            (utxo_id, coin)
        })
        .collect();

    coins
}

// Setup a test client with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
pub async fn setup_test_client(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Config,
) -> (FuelClient, SocketAddr) {
    let coin_configs = coins
        .into_iter()
        .map(|(utxo_id, coin)| CoinConfig {
            tx_id: Some(*utxo_id.tx_id()),
            output_index: Some(utxo_id.output_index() as u64),
            block_created: Some(coin.block_created),
            maturity: Some(coin.maturity),
            owner: coin.owner,
            amount: coin.amount,
            asset_id: coin.asset_id,
        })
        .collect();

    // Setup node config with genesis coins and utxo_validation enabled
    let config = Config {
        chain_conf: ChainConfig {
            initial_state: Some(StateConfig {
                coins: Some(coin_configs),
                ..StateConfig::default()
            }),
            ..ChainConfig::local_testnet()
        },
        database_type: DbType::InMemory,
        utxo_validation: true,
        ..node_config
    };

    let srv = FuelService::new_node(config).await.unwrap();
    let client = FuelClient::from(srv.bound_address);

    (client, srv.bound_address)
}

// TODO Emir make this optional
// #[cfg(feature="Emily")]
pub async fn setup_test_client_bin(
    coins: Vec<(UtxoId, Coin)>,
    // node_config: Config
) -> (Child, FuelClient, SocketAddr) {
    let coin_configs: Vec<String> = coins
        .into_iter()
        .map(|(utxo_id, coin)| {
            serde_json::to_string(&DummyConfig {
                tx_id: Some(*utxo_id.tx_id()),
                output_index: Some(utxo_id.output_index() as u64),
                block_created: Some(coin.block_created),
                maturity: Some(coin.maturity),
                owner: coin.owner,
                amount: coin.amount,
                asset_id: coin.asset_id,
            })
            .unwrap()
        })
        .collect();

    let config_with_coins: Value = serde_json::from_str(coin_configs.concat().as_str()).unwrap();

    let _ = get_node_config_json(config_with_coins);

    let fuel_core_bin = env::var("FUEL_CORE_BIN").unwrap_or_else(|_| "FUEL_CORE_BIN".to_string());
    let fuel_core_config = env::var("FUEL_CORE_CONFIG").unwrap_or_else(|_| "FUEL_CORE_CONFIG".to_string());

    let running_node = Command::new(fuel_core_bin)
        .arg("--ip")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("4000")
        .arg("--chain")
        .arg(fuel_core_config)
        .arg("--db-type")
        .arg("in-memory")
        .spawn()
        .expect("FUEL_CORE_BIN is unable to find. Please set FUEL_CORE_BIN");

    sleep(Duration::from_secs(2));

    let srv_address = SocketAddr::new("127.0.0.1".parse().unwrap(), 4000);
    let client = FuelClient::from(srv_address);

    (running_node, client, srv_address)
}
