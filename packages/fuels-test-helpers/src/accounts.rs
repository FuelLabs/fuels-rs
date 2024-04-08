use std::mem::size_of;

use fuel_crypto::SecretKey;
use fuels_accounts::wallet::WalletUnlocked;
use fuels_core::types::errors::Result;

use crate::{
    node_types::{ChainConfig, Config},
    setup_custom_assets_coins, setup_test_provider,
    wallets_config::*,
};

/// Launches a local Fuel node, instantiates a provider, and returns a wallet.
/// The provider and the wallets are instantiated with the default configs.
/// For more configurable options, see the `launch_custom_provider_and_get_wallets` function.
/// # Examples
/// ```
/// use fuels_test_helpers::launch_provider_and_get_wallet;
///
/// async fn single_wallet() -> Result<(), Box<dyn std::error::Error>> {
///   let wallet = launch_provider_and_get_wallet().await?;
///   dbg!(wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_provider_and_get_wallet() -> Result<WalletUnlocked> {
    let mut wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::new(Some(1), None, None), None, None)
            .await?;

    Ok(wallets.pop().expect("should have one wallet"))
}

/// Launches a custom node and provider, along with a configurable number of wallets.
///
/// # Examples
/// ```
/// use fuels_test_helpers::launch_custom_provider_and_get_wallets;
/// use fuels_test_helpers::WalletsConfig;
///
/// async fn multiple_wallets() -> Result<(), Box<dyn std::error::Error>> {
///   let num_wallets = 2;
///   let num_coins = 1;
///   let amount = 1;
///   let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));
///
///   let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;
///   let first_wallet = wallets.pop().unwrap();
///   dbg!(first_wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_custom_provider_and_get_wallets(
    wallet_config: WalletsConfig,
    provider_config: Option<Config>,
    chain_config: Option<ChainConfig>,
) -> Result<Vec<WalletUnlocked>> {
    const SIZE_SECRET_KEY: usize = size_of::<SecretKey>();
    const PADDING_BYTES: usize = SIZE_SECRET_KEY - size_of::<u64>();
    let mut secret_key: [u8; SIZE_SECRET_KEY] = [0; SIZE_SECRET_KEY];

    let mut wallets: Vec<_> = (1..=wallet_config.num_wallets())
        .map(|wallet_counter| {
            secret_key[PADDING_BYTES..].copy_from_slice(&wallet_counter.to_be_bytes());

            WalletUnlocked::new_from_private_key(
                SecretKey::try_from(secret_key.as_slice())
                    .expect("This should never happen as we provide a [u8; SIZE_SECRET_KEY] array"),
                None,
            )
        })
        .collect();

    let all_coins = wallets
        .iter()
        .flat_map(|wallet| setup_custom_assets_coins(wallet.address(), wallet_config.assets()))
        .collect::<Vec<_>>();

    let provider = setup_test_provider(all_coins, vec![], provider_config, chain_config).await?;

    for wallet in &mut wallets {
        wallet.set_provider(provider.clone());
    }

    Ok(wallets)
}

#[cfg(test)]
mod tests {
    use fuel_core_chain_config::ChainConfig;
    use fuel_tx::{ConsensusParameters, TxParameters};
    use fuel_types::AssetId;
    use fuels_accounts::ViewOnlyAccount;
    use fuels_core::{
        constants::BASE_ASSET_ID,
        types::{coin_type::CoinType, errors::Result},
    };
    use rand::Fill;

    use crate::{launch_custom_provider_and_get_wallets, AssetConfig, WalletsConfig};

    #[tokio::test]
    async fn test_wallet_config() -> Result<()> {
        let num_wallets = 2;
        let num_coins = 3;
        let amount = 100;
        let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));

        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;

        assert_eq!(wallets.len(), num_wallets as usize);

        for wallet in &wallets {
            let coins = wallet.get_coins(BASE_ASSET_ID).await?;

            assert_eq!(coins.len(), num_coins as usize);

            for coin in &coins {
                assert_eq!(coin.amount, amount);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_wallet_config_multiple_assets(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut rng = rand::thread_rng();
        let num_wallets = 3;

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

        let config = WalletsConfig::new_multiple_assets(num_wallets, assets.clone());
        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;
        assert_eq!(wallets.len(), num_wallets as usize);

        for asset in assets {
            for wallet in &wallets {
                let resources = wallet
                    .get_spendable_resources(asset.id, asset.num_coins * asset.coin_amount)
                    .await?;
                assert_eq!(resources.len() as u64, asset.num_coins);

                for resource in resources {
                    assert_eq!(resource.amount(), asset.coin_amount);
                    match resource {
                        CoinType::Coin(coin) => {
                            assert_eq!(&coin.owner, wallet.address())
                        }
                        CoinType::Message(_) => panic!("resources contained messages"),
                    }
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn generated_wallets_are_deterministic() -> Result<()> {
        let num_wallets = 32;
        let num_coins = 1;
        let amount = 100;
        let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));

        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;

        assert_eq!(
            wallets.get(31).unwrap().address().to_string(),
            "fuel1rsjlwjzx0px3zu2al05jdlzp4j5quqzlk7pzyk4g45x6m7r3elzsz9dwh4".to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn generated_wallets_with_custom_chain_config() -> Result<()> {
        let consensus_parameters = ConsensusParameters {
            tx_params: TxParameters::default().with_max_gas_per_tx(10_000_000_000),
            ..Default::default()
        };

        let block_gas_limit = 10_000_000_000;
        let chain_config = ChainConfig {
            consensus_parameters,
            block_gas_limit,
            ..ChainConfig::default()
        };

        let num_wallets = 4;
        let num_coins = 3;
        let coin_amount = 2_000_000_000;
        let wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(coin_amount)),
            None,
            Some(chain_config),
        )
        .await?;

        assert_eq!(wallets.len() as u64, num_wallets);

        for wallet in wallets.into_iter() {
            assert_eq!(
                wallet
                    .try_provider()?
                    .consensus_parameters()
                    .tx_params()
                    .max_gas_per_tx,
                block_gas_limit
            );
            assert_eq!(
                wallet.get_coins(AssetId::default()).await?.len() as u64,
                num_coins
            );
            assert_eq!(
                *wallet
                    .get_balances()
                    .await?
                    .get("0000000000000000000000000000000000000000000000000000000000000000")
                    .expect("failed to get value"),
                num_coins * coin_amount
            );
        }

        Ok(())
    }
}
