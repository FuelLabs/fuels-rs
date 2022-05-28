//! Testing helpers/utilities for Fuel SDK.

pub use fuel_core::service::Config;
use fuel_core::{
    chain_config::{ChainConfig, CoinConfig, StateConfig},
    model::{Coin, CoinStatus},
    service::{DbType, FuelService},
};
use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Address, Bytes32, UtxoId},
};
use fuels_signers::fuel_crypto::fuel_types::AssetId;
use rand::Fill;
use std::net::SocketAddr;

#[cfg(feature = "fuels-signers")]
mod signers;
mod wallets_config;

#[cfg(feature = "fuels-signers")]
pub use signers::*;
pub use wallets_config::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_setup_single_asset_coins() {
        let mut rng = rand::thread_rng();
        let mut address = Address::zeroed();
        address.try_fill(&mut rng).unwrap();
        let mut asset_id = AssetId::zeroed();
        asset_id.try_fill(&mut rng).unwrap();
        let coins = setup_single_asset_coins(address, asset_id, 11, 10);
        assert_eq!(coins.len(), 11);
        for (_utxo_id, coin) in coins {
            assert_eq!(coin.asset_id, asset_id);
            assert_eq!(coin.amount, 10);
            assert_eq!(coin.owner, address);
        }
    }
}
