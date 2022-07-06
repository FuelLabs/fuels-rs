//! Testing helpers/utilities for Fuel SDK.

use std::collections::HashSet;
use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::{
    chain_config::{ChainConfig, CoinConfig, StateConfig},
    model::{Coin, CoinStatus},
    service::{DbType, FuelService},
};

#[cfg(feature = "fuel-core-lib")]
pub use fuel_core::service::Config;

#[cfg(not(feature = "fuel-core-lib"))]
pub use node::{CoinConfig, Config};

#[cfg(not(feature = "fuel-core-lib"))]
use fuel_core_interfaces::model::{Coin, CoinStatus};

#[cfg(not(feature = "fuel-core-lib"))]
use portpicker::{is_free, pick_unused_port};

#[cfg(not(feature = "fuel-core-lib"))]
use serde_json::Value;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::node::spawn_fuel_service;

use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Address, Bytes32, UtxoId},
};

use fuels_core::constants::BASE_ASSET_ID;
use fuels_signers::fuel_crypto::fuel_types::AssetId;
use fuels_signers::fuel_crypto::rand;
use rand::Fill;

#[cfg(not(feature = "fuel-core-lib"))]
mod node;

mod chains;
mod script;
#[cfg(feature = "fuels-signers")]
mod signers;
mod utils;
mod wallets_config;

#[cfg(not(feature = "fuel-core-lib"))]
pub use node::*;

pub use chains::*;
#[cfg(feature = "fuels-signers")]
pub use signers::*;
pub use wallets_config::*;

/// Create a vector of `num_asset`*`coins_per_asset` UTXOs and a vector of the unique corresponding
/// asset IDs. `AssetId`. Each UTXO (=coin) contains `amount_per_coin` amount of a random asset. The
/// output of this function can be used with `setup_test_client` to get a client with some
/// pre-existing coins, with `num_asset` different asset ids. Note that one of the assets is the
/// base asset to pay for gas.
pub fn setup_multiple_assets_coins(
    owner: Address,
    num_asset: u64,
    coins_per_asset: u64,
    amount_per_coin: u64,
) -> (Vec<(UtxoId, Coin)>, Vec<AssetId>) {
    let mut rng = rand::thread_rng();
    // Create `num_asset-1` asset ids so there is `num_asset` in total with the base asset
    let mut coins = (0..(num_asset - 1))
        .flat_map(|_| {
            let mut random_asset_id = AssetId::zeroed();
            random_asset_id.try_fill(&mut rng).unwrap();
            setup_single_asset_coins(owner, random_asset_id, coins_per_asset, amount_per_coin)
        })
        .collect::<Vec<(UtxoId, Coin)>>();
    // Add the base asset
    coins.extend(setup_single_asset_coins(
        owner,
        BASE_ASSET_ID,
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

// Setup a test client with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuel-core-lib")]
pub async fn setup_test_client(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Option<Config>,
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
        ..node_config.unwrap_or_else(Config::local_node)
    };

    let srv = FuelService::new_node(config).await.unwrap();
    let client = FuelClient::from(srv.bound_address);

    (client, srv.bound_address)
}

#[cfg(not(feature = "fuel-core-lib"))]
pub async fn setup_test_client(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Option<Config>,
) -> (FuelClient, SocketAddr) {
    let coin_configs: Vec<Value> = coins
        .into_iter()
        .map(|(utxo_id, coin)| {
            serde_json::to_value(&CoinConfig {
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

    let result = serde_json::to_string(&coin_configs).expect("Failed to stringify coins vector");

    let config_with_coins: Value =
        serde_json::from_str(result.as_str()).expect("Failed to build config_with_coins JSON");

    let srv_address = match node_config {
        Some(config) if config.addr.port() != 0 && is_free(config.addr.port()) => config.addr,
        Some(config) if !is_free(config.addr.port()) => panic!("Error: Address already in use"),
        _ => get_socket_address(),
    };

    spawn_fuel_service(config_with_coins, srv_address.port());

    let client = FuelClient::from(srv_address);

    server_health_check(&client).await;

    (client, srv_address)
}

#[cfg(not(feature = "fuel-core-lib"))]
fn get_socket_address() -> SocketAddr {
    let free_port = pick_unused_port().expect("No ports free");
    SocketAddr::new("127.0.0.1".parse().unwrap(), free_port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_signers::{LocalWallet, Signer};
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_setup_single_asset_coins() -> Result<(), rand::Error> {
        let mut rng = rand::thread_rng();
        let mut address = Address::zeroed();
        address.try_fill(&mut rng)?;
        let mut asset_id = AssetId::zeroed();
        asset_id.try_fill(&mut rng)?;
        let number_of_coins = 11;
        let amount_per_coin = 10;
        let coins = setup_single_asset_coins(address, asset_id, number_of_coins, amount_per_coin);
        assert_eq!(coins.len() as u64, number_of_coins);
        for (_utxo_id, coin) in coins {
            assert_eq!(coin.asset_id, asset_id);
            assert_eq!(coin.amount, amount_per_coin);
            assert_eq!(coin.owner, address);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_setup_multiple_assets_coins() -> Result<(), rand::Error> {
        let mut rng = rand::thread_rng();
        let mut address = Address::zeroed();
        address.try_fill(&mut rng)?;
        let number_of_assets = 7;
        let coins_per_asset = 10;
        let amount_per_coin = 13;
        let (coins, unique_asset_ids) = setup_multiple_assets_coins(
            address,
            number_of_assets,
            coins_per_asset,
            amount_per_coin,
        );
        assert_eq!(coins.len() as u64, number_of_assets * coins_per_asset);
        assert_eq!(unique_asset_ids.len() as u64, number_of_assets);
        // Check that the wallet has base assets to pay for gas
        assert!(unique_asset_ids
            .iter()
            .any(|&asset_id| asset_id == BASE_ASSET_ID));
        for asset_id in unique_asset_ids {
            let coins_asset_id: Vec<(UtxoId, Coin)> = coins
                .clone()
                .into_iter()
                .filter(|(_, c)| c.asset_id == asset_id)
                .collect();
            assert_eq!(coins_asset_id.len() as u64, coins_per_asset);
            for (_utxo_id, coin) in coins_asset_id {
                assert_eq!(coin.owner, address);
                assert_eq!(coin.amount, amount_per_coin);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_setup_test_client_custom_config() -> Result<(), rand::Error> {
        let socket = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 5000);

        let wallet = LocalWallet::new_random(None);

        let coins: Vec<(UtxoId, Coin)> = setup_single_asset_coins(
            wallet.address(),
            Default::default(),
            DEFAULT_NUM_COINS,
            DEFAULT_COIN_AMOUNT,
        );

        let config = Config {
            addr: socket,
            ..Config::local_node()
        };

        let wallets = setup_test_client(coins, Some(config)).await;

        assert_eq!(wallets.1, socket);
        Ok(())
    }
}
