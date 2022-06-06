//! Testing helpers/utilities for Fuel SDK using fuel-core-bin.

use std::borrow::Borrow;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

use fuel_core_interfaces::model::{Coin, CoinStatus};
use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Address, Bytes32, UtxoId},
};
use rand::{Fill, Rng};
use serde_json::Value;
use tokio::process::Command;

use fuels_core::constants::NATIVE_ASSET_ID;
use fuels_signers::fuel_crypto::fuel_types::AssetId;
use fuels_signers::fuel_crypto::rand;
pub use signers_bin::*;

use crate::node_config_json::{get_node_config_json, DummyConfig, FuelCoreServer};

mod node_config_json;

mod signers_bin;

mod wallets_config_bin;

/// Create a vector of `num_asset`*`coins_per_asset` UTXOs and a vector of the unique corresponding
/// asset IDs. `AssetId`. Each UTXO (=coin) contains `amount_per_coin` amount of a random asset. The
/// output of this function can be used with `setup_test_client` to get a client with some
/// pre-existing coins, with `num_asset` different asset ids. Note that one of the assets is the
/// native asset to pay for gas.
pub fn setup_multiple_assets_coins(
    owner: Address,
    num_asset: u64,
    coins_per_asset: u64,
    amount_per_coin: u64,
) -> (Vec<(UtxoId, Coin)>, Vec<AssetId>) {
    let mut rng = rand::thread_rng();
    // Create `num_asset-1` asset ids so there is `num_asset` in total with the native asset
    let mut coins = (0..(num_asset - 1))
        .flat_map(|_| {
            let mut random_asset_id = AssetId::zeroed();
            random_asset_id.try_fill(&mut rng).unwrap();
            setup_single_asset_coins(owner, random_asset_id, coins_per_asset, amount_per_coin)
        })
        .collect::<Vec<(UtxoId, Coin)>>();
    // Add the native asset
    coins.extend(setup_single_asset_coins(
        owner,
        NATIVE_ASSET_ID,
        coins_per_asset,
        amount_per_coin,
    ));
    let asset_ids = coins
        .clone()
        .into_iter()
        .map(|(_utxo_id, coin)| coin.asset_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<AssetId>>();
    (coins, asset_ids)
}

/// Create a vector of `num_coins` UTXOs containing `amount_per_coin` amount of asset `asset_id`.
/// The output of this function can be used with `setup_test_client` to get a client with some
/// pre-existing coins, but with only one asset ID.
pub fn setup_single_asset_coins(
    owner: Address,
    asset_id: AssetId,
    num_coins: u64,
    amount_per_coin: u64,
) -> Vec<(UtxoId, Coin)> {
    let mut rng = rand::thread_rng();

    let coins: Vec<(UtxoId, Coin)> = (1..=num_coins)
        .map(|_i| {
            let coin = Coin {
                owner,
                amount: amount_per_coin,
                asset_id,
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

pub async fn setup_test_client_bin(
    coins: Vec<(UtxoId, Coin)>,
    // node_config: Config
) -> (FuelCoreServer, FuelClient, SocketAddr) {
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
    let temp_config_file = get_node_config_json(config_with_coins);
    // ports should be checked
    let free_port = rand::thread_rng().gen_range(4000..9000);

    let running_node = Command::new("fuel-core")
        .arg("--ip")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(&free_port.to_string())
        .arg("--chain")
        .arg(temp_config_file.borrow().path())
        .arg("--db-type")
        .arg("in-memory")
        .kill_on_drop(true)
        .spawn()
        .expect("Could not find 'fuel-core' in PATH. Please check if it's installed");

    let fuel_core_server = FuelCoreServer {
        process_handle: running_node,
        config_file: temp_config_file,
    };

    sleep(Duration::from_secs(2));

    let srv_address = SocketAddr::new("127.0.0.1".parse().unwrap(), free_port);
    let client = FuelClient::from(srv_address);

    (fuel_core_server, client, srv_address)
}
