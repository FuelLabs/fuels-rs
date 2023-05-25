//! Testing helpers/utilities for Fuel SDK.
extern crate core;

use std::net::SocketAddr;

#[cfg(feature = "fuels-accounts")]
pub use accounts::*;
#[cfg(feature = "fuel-core-lib")]
pub use fuel_core::service::Config;
#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::FuelService;
use fuel_core_chain_config::ChainConfig;
#[cfg(feature = "fuel-core-lib")]
use fuel_core_chain_config::StateConfig;
use fuel_core_client::client::FuelClient;
use fuel_tx::{Bytes32, ConsensusParameters, UtxoId};
use fuel_types::{AssetId, Nonce};
use fuels_types::coin::CoinStatus;
use fuels_types::{
    bech32::Bech32Address,
    coin::Coin,
    constants::BASE_ASSET_ID,
    message::{Message, MessageStatus},
};
#[cfg(not(feature = "fuel-core-lib"))]
pub use node::*;
#[cfg(not(feature = "fuel-core-lib"))]
use portpicker::is_free;
use rand::Fill;
#[cfg(feature = "fuel-core-lib")]
pub use utils::{into_coin_configs, into_message_configs};
pub use wallets_config::*;

#[cfg(not(feature = "fuel-core-lib"))]
pub mod node;

#[cfg(feature = "fuels-accounts")]
mod accounts;
mod utils;
mod wallets_config;

/// Create a vector of `num_asset`*`coins_per_asset` UTXOs and a vector of the unique corresponding
/// asset IDs. `AssetId`. Each UTXO (=coin) contains `amount_per_coin` amount of a random asset. The
/// output of this function can be used with `setup_test_client` to get a client with some
/// pre-existing coins, with `num_asset` different asset ids. Note that one of the assets is the
/// base asset to pay for gas.
pub fn setup_multiple_assets_coins(
    owner: &Bech32Address,
    num_asset: u64,
    coins_per_asset: u64,
    amount_per_coin: u64,
) -> (Vec<Coin>, Vec<AssetId>) {
    let mut rng = rand::thread_rng();
    // Create `num_asset-1` asset ids so there is `num_asset` in total with the base asset
    let asset_ids = (0..(num_asset - 1))
        .map(|_| {
            let mut random_asset_id = AssetId::zeroed();
            random_asset_id.try_fill(&mut rng).unwrap();
            random_asset_id
        })
        .chain([BASE_ASSET_ID].into_iter())
        .collect::<Vec<AssetId>>();

    let coins = asset_ids
        .iter()
        .flat_map(|id| setup_single_asset_coins(owner, *id, coins_per_asset, amount_per_coin))
        .collect::<Vec<Coin>>();

    (coins, asset_ids)
}

/// Create a vector of UTXOs with the provided AssetIds, num_coins, and amount_per_coin
pub fn setup_custom_assets_coins(owner: &Bech32Address, assets: &[AssetConfig]) -> Vec<Coin> {
    let coins = assets
        .iter()
        .flat_map(|asset| {
            setup_single_asset_coins(owner, asset.id, asset.num_coins, asset.coin_amount)
        })
        .collect::<Vec<Coin>>();
    coins
}

/// Create a vector of `num_coins` UTXOs containing `amount_per_coin` amount of asset `asset_id`.
/// The output of this function can be used with `setup_test_client` to get a client with some
/// pre-existing coins, but with only one asset ID.
pub fn setup_single_asset_coins(
    owner: &Bech32Address,
    asset_id: AssetId,
    num_coins: u64,
    amount_per_coin: u64,
) -> Vec<Coin> {
    let mut rng = rand::thread_rng();

    let coins: Vec<Coin> = (1..=num_coins)
        .map(|_i| {
            let mut r = Bytes32::zeroed();
            r.try_fill(&mut rng).unwrap();
            let utxo_id = UtxoId::new(r, 0);

            Coin {
                owner: owner.clone(),
                utxo_id,
                amount: amount_per_coin,
                asset_id,
                maturity: Default::default(),
                status: CoinStatus::Unspent,
                block_created: Default::default(),
            }
        })
        .collect();

    coins
}

pub fn setup_single_message(
    sender: &Bech32Address,
    recipient: &Bech32Address,
    amount: u64,
    nonce: Nonce,
    data: Vec<u8>,
) -> Message {
    Message {
        sender: sender.clone(),
        recipient: recipient.clone(),
        nonce,
        amount,
        data,
        da_height: 0,
        status: MessageStatus::Unspent,
    }
}

// Setup a test client with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuel-core-lib")]
#[allow(clippy::let_unit_value)]
pub async fn setup_test_client(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    node_config: Option<Config>,
    chain_config: Option<ChainConfig>,
) -> (FuelClient, SocketAddr, ConsensusParameters) {
    let coin_configs = into_coin_configs(coins);
    let message_configs = into_message_configs(messages);
    let mut chain_conf = chain_config.unwrap_or_else(ChainConfig::local_testnet);

    chain_conf.initial_state = Some(StateConfig {
        coins: Some(coin_configs),
        contracts: None,
        messages: Some(message_configs),
        ..StateConfig::default()
    });

    let mut config = node_config.unwrap_or_else(Config::local_node);
    config.chain_conf = chain_conf;

    let srv = FuelService::new_node(config).await.unwrap();
    let address = srv.bound_address;
    tokio::spawn(async move {
        let _own_the_handle = srv;
        let () = futures::future::pending().await;
    });

    let client = FuelClient::from(address);
    let consensus_parameters = client
        .chain_info()
        .await
        .expect("Could not retrieve consensus parameters of running client, should not happen")
        .consensus_parameters
        .into();

    (client, address, consensus_parameters)
}

#[cfg(not(feature = "fuel-core-lib"))]
pub async fn setup_test_client(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    node_config: Option<Config>,
    chain_config: Option<ChainConfig>,
) -> (FuelClient, SocketAddr, ConsensusParameters) {
    let config = node_config.unwrap_or_else(Config::local_node);
    let requested_port = config.addr.port();

    let bound_address = if requested_port == 0 {
        get_socket_address()
    } else if is_free(requested_port) {
        config.addr
    } else {
        panic!("Error: Address already in use");
    };

    new_fuel_node(
        coins,
        messages,
        Config {
            addr: bound_address,
            ..config
        },
        chain_config,
    )
    .await;

    let client = FuelClient::from(bound_address);
    server_health_check(&client).await;

    let consensus_parameters = client
        .chain_info()
        .await
        .expect("Could not retrieve consensus parameters of running client should not happen")
        .consensus_parameters
        .into();

    (client, bound_address, consensus_parameters)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use fuels_types::bech32::FUEL_BECH32_HRP;

    use super::*;

    #[tokio::test]
    async fn test_setup_single_asset_coins() -> Result<(), rand::Error> {
        let mut rng = rand::thread_rng();
        let mut addr_data = Bytes32::new([0u8; 32]);
        addr_data.try_fill(&mut rng)?;
        let address = Bech32Address::new("test", addr_data);

        let mut asset_id = AssetId::zeroed();
        asset_id.try_fill(&mut rng)?;

        let number_of_coins = 11;
        let amount_per_coin = 10;
        let coins = setup_single_asset_coins(&address, asset_id, number_of_coins, amount_per_coin);

        assert_eq!(coins.len() as u64, number_of_coins);
        for coin in coins {
            assert_eq!(coin.asset_id, asset_id);
            assert_eq!(coin.amount, amount_per_coin);
            assert_eq!(coin.owner, address);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_setup_multiple_assets_coins() -> Result<(), rand::Error> {
        let mut rng = rand::thread_rng();
        let mut addr_data = Bytes32::new([0u8; 32]);
        addr_data.try_fill(&mut rng)?;
        let address = Bech32Address::new("test", addr_data);

        let number_of_assets = 7;
        let coins_per_asset = 10;
        let amount_per_coin = 13;
        let (coins, unique_asset_ids) = setup_multiple_assets_coins(
            &address,
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
            let coins_asset_id: Vec<Coin> = coins
                .clone()
                .into_iter()
                .filter(|c| c.asset_id == asset_id)
                .collect();
            assert_eq!(coins_asset_id.len() as u64, coins_per_asset);
            for coin in coins_asset_id {
                assert_eq!(coin.owner, address);
                assert_eq!(coin.amount, amount_per_coin);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_setup_custom_assets_coins() -> Result<(), rand::Error> {
        let mut rng = rand::thread_rng();
        let mut hash = [0u8; 32];
        hash.try_fill(&mut rng)?;
        let address = Bech32Address::new(FUEL_BECH32_HRP, hash);

        let asset_base = AssetConfig {
            id: BASE_ASSET_ID,
            num_coins: 2,
            coin_amount: 4,
        };

        let mut asset_id_1 = AssetId::zeroed();
        asset_id_1.try_fill(&mut rng)?;
        let asset_1 = AssetConfig {
            id: asset_id_1,
            num_coins: 6,
            coin_amount: 8,
        };

        let mut asset_id_2 = AssetId::zeroed();
        asset_id_2.try_fill(&mut rng)?;
        let asset_2 = AssetConfig {
            id: asset_id_2,
            num_coins: 10,
            coin_amount: 12,
        };

        let assets = vec![asset_base, asset_1, asset_2];
        let coins = setup_custom_assets_coins(&address, &assets);

        for asset in assets {
            let coins_asset_id: Vec<Coin> = coins
                .clone()
                .into_iter()
                .filter(|c| c.asset_id == asset.id)
                .collect();
            assert_eq!(coins_asset_id.len() as u64, asset.num_coins);
            for coin in coins_asset_id {
                assert_eq!(coin.owner, address);
                assert_eq!(coin.amount, asset.coin_amount);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_setup_test_client_custom_config() -> Result<(), rand::Error> {
        let socket = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 4000);
        let config = Config {
            addr: socket,
            utxo_validation: true,
            manual_blocks_enabled: true,
            ..Config::local_node()
        };

        let (client, bound_addr, _consensus_parameters) =
            setup_test_client(vec![], vec![], Some(config.clone()), None).await;
        let node_info = client
            .node_info()
            .await
            .expect("Failed to retrieve node info!");

        assert_eq!(bound_addr, socket);
        assert_eq!(node_info.utxo_validation, config.utxo_validation);

        Ok(())
    }

    #[tokio::test]
    async fn test_setup_test_client_consensus_parameters_config() {
        let configured_parameters = ConsensusParameters::DEFAULT
            .with_max_gas_per_tx(2)
            .with_gas_per_byte(2)
            .with_max_inputs(58)
            .with_max_storage_slots(83);

        let chain_config = ChainConfig {
            transaction_parameters: configured_parameters,
            ..ChainConfig::default()
        };
        let (client, _, client_consensus_parameters) =
            setup_test_client(vec![], vec![], None, Some(chain_config)).await;

        let retrieved_parameters: ConsensusParameters = client
            .chain_info()
            .await
            .expect("Failed to retrieve consensus chain info!")
            .consensus_parameters
            .into();

        assert_eq!(retrieved_parameters, configured_parameters);
        assert_eq!(client_consensus_parameters, configured_parameters)
    }

    #[tokio::test]
    async fn test_chain_config_and_consensus_parameters() {
        let consensus_parameters_config = ConsensusParameters::DEFAULT
            .with_max_inputs(123)
            .with_gas_per_byte(456);

        let chain_config = ChainConfig {
            chain_name: "Solo_Munib".to_string(),
            transaction_parameters: consensus_parameters_config,
            ..ChainConfig::local_testnet()
        };

        let (fuel_client, _, client_consensus_parameters) =
            setup_test_client(vec![], vec![], None, Some(chain_config)).await;

        let chain_info = fuel_client.chain_info().await.unwrap();

        assert_eq!(chain_info.name, "Solo_Munib");
        assert_eq!(chain_info.consensus_parameters.max_inputs.0, 123);
        assert_eq!(chain_info.consensus_parameters.gas_per_byte.0, 456);
        assert_eq!(client_consensus_parameters.gas_per_byte, 456);
        assert_eq!(client_consensus_parameters.max_inputs, 123);
    }
}
