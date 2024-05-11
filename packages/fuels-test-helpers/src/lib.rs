//! Testing helpers/utilities for Fuel SDK.
#[cfg(feature = "std")]
pub use accounts::*;
pub use node_types::*;
#[cfg(feature = "std")]
pub use service::*;
#[cfg(feature = "std")]
pub use utils::*;
pub use wallets_config::*;

#[cfg(feature = "std")]
mod accounts;
#[cfg(all(not(feature = "fuel-core-lib"), feature = "std"))]
pub(crate) mod fuel_bin_service;
mod node_types;
#[cfg(feature = "std")]
mod service;
#[cfg(feature = "std")]
mod utils;
mod wallets_config;

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use fuel_tx::{
        AssetId, Bytes32, ConsensusParameters, ContractParameters, FeeParameters, TxParameters,
    };
    use fuels_core::types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        coin::Coin,
        errors::Result,
    };
    use rand::Fill;

    use super::*;

    #[tokio::test]
    async fn test_setup_single_asset_coins() -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut addr_data = Bytes32::new([0u8; 32]);
        addr_data
            .try_fill(&mut rng)
            .expect("failed to fill with random data");
        let address = Bech32Address::new("test", addr_data);

        let mut asset_id = AssetId::zeroed();
        asset_id
            .try_fill(&mut rng)
            .expect("failed to fill with random data");

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
    async fn test_setup_multiple_assets_coins() -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut addr_data = Bytes32::new([0u8; 32]);
        addr_data
            .try_fill(&mut rng)
            .expect("failed to fill with random data");
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
            .any(|&asset_id| asset_id == AssetId::zeroed()));
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
    async fn test_setup_custom_assets_coins() -> Result<()> {
        let mut rng = rand::thread_rng();
        let mut hash = [0u8; 32];
        hash.try_fill(&mut rng)
            .expect("failed to fill with random data");
        let address = Bech32Address::new(FUEL_BECH32_HRP, hash);

        let asset_base = AssetConfig {
            id: AssetId::zeroed(),
            num_coins: 2,
            coin_amount: 4,
        };

        let mut asset_id_1 = AssetId::zeroed();
        asset_id_1
            .try_fill(&mut rng)
            .expect("failed to fill with random data");
        let asset_1 = AssetConfig {
            id: asset_id_1,
            num_coins: 6,
            coin_amount: 8,
        };

        let mut asset_id_2 = AssetId::zeroed();
        asset_id_2
            .try_fill(&mut rng)
            .expect("failed to fill with random data");
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
    async fn test_setup_test_provider_custom_config() -> Result<()> {
        let socket = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 4000);
        let config = NodeConfig {
            addr: socket,
            ..NodeConfig::default()
        };

        let provider = setup_test_provider(vec![], vec![], Some(config.clone()), None).await?;
        let node_info = provider
            .node_info()
            .await
            .expect("Failed to retrieve node info!");

        assert_eq!(provider.url(), format!("http://127.0.0.1:4000"));
        assert_eq!(node_info.utxo_validation, config.utxo_validation);

        Ok(())
    }

    #[tokio::test]
    async fn test_setup_test_client_consensus_parameters_config() -> Result<()> {
        let tx_params = TxParameters::default()
            .with_max_gas_per_tx(2)
            .with_max_inputs(58);
        let fee_params = FeeParameters::default().with_gas_per_byte(2);
        let contract_params = ContractParameters::default().with_max_storage_slots(83);

        let mut consensus_parameters = ConsensusParameters::default();
        consensus_parameters.set_tx_params(tx_params);
        consensus_parameters.set_fee_params(fee_params);
        consensus_parameters.set_contract_params(contract_params);

        let chain_config = ChainConfig {
            consensus_parameters: consensus_parameters.clone(),
            ..ChainConfig::default()
        };
        let provider = setup_test_provider(vec![], vec![], None, Some(chain_config)).await?;

        let retrieved_parameters = provider.consensus_parameters();

        assert_eq!(*retrieved_parameters, consensus_parameters);

        Ok(())
    }

    #[tokio::test]
    async fn test_chain_config_and_consensus_parameters() -> Result<()> {
        let max_inputs = 123;
        let gas_per_byte = 456;

        let mut consensus_parameters = ConsensusParameters::default();

        let tx_params = TxParameters::default().with_max_inputs(max_inputs);
        consensus_parameters.set_tx_params(tx_params);

        let fee_params = FeeParameters::default().with_gas_per_byte(gas_per_byte);
        consensus_parameters.set_fee_params(fee_params);

        let chain_name = "fuel-0".to_string();
        let chain_config = ChainConfig {
            chain_name: chain_name.clone(),
            consensus_parameters,
            ..ChainConfig::local_testnet()
        };

        let provider = setup_test_provider(vec![], vec![], None, Some(chain_config)).await?;

        let chain_info = provider.chain_info().await?;

        assert_eq!(chain_info.name, chain_name);
        assert_eq!(
            chain_info.consensus_parameters.tx_params().max_inputs(),
            max_inputs
        );
        assert_eq!(
            chain_info.consensus_parameters.fee_params().gas_per_byte(),
            gas_per_byte
        );
        Ok(())
    }
}
