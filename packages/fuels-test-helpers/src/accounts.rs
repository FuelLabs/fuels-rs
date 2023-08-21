use std::{mem::size_of, net::SocketAddr};

#[cfg(feature = "fuel-core-lib")]
use fuel_core::service::Config;
use fuel_core_chain_config::ChainConfig;
use fuels_accounts::{
    fuel_crypto::SecretKey, provider::Provider, wallet::WalletUnlocked, ViewOnlyAccount,
};
use fuels_core::types::{coin::Coin, message::Message};

#[cfg(not(feature = "fuel-core-lib"))]
use crate::node::Config;
use crate::{setup_custom_assets_coins, setup_test_client, wallets_config::*};

/// Launches a local Fuel node, instantiates a provider, and returns a wallet.
/// The provider and the wallets are instantiated with the default configs.
/// For more configurable options, see the `launch_custom_provider_and_get_wallets` function.
/// # Examples
/// ```
/// use fuels_test_helpers::launch_provider_and_get_wallet;
/// use fuels_accounts::Signer;
///
/// async fn single_wallet() -> Result<(), Box<dyn std::error::Error>> {
///   let wallet = launch_provider_and_get_wallet().await;
///   dbg!(wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_provider_and_get_wallet() -> WalletUnlocked {
    let mut wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::new(Some(1), None, None), None, None)
            .await;

    wallets.pop().unwrap()
}

/// Launches a custom node and provider, along with a configurable number of wallets.
///
/// # Examples
/// ```
/// use fuels_test_helpers::launch_custom_provider_and_get_wallets;
/// use fuels_accounts::Signer;
/// use fuels_test_helpers::WalletsConfig;
///
/// async fn multiple_wallets() -> Result<(), Box<dyn std::error::Error>> {
///   let num_wallets = 2;
///   let num_coins = 1;
///   let amount = 1;
///   let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));
///
///   let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
///   let first_wallet = wallets.pop().unwrap();
///   dbg!(first_wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_custom_provider_and_get_wallets(
    wallet_config: WalletsConfig,
    provider_config: Option<Config>,
    chain_config: Option<ChainConfig>,
) -> Vec<WalletUnlocked> {
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

    let (provider, _) = setup_test_provider(all_coins, vec![], provider_config, chain_config).await;

    for wallet in &mut wallets {
        wallet.set_provider(provider.clone());
    }

    wallets
}

/// Setup a test provider with the given coins. We return the SocketAddr so the launched node
/// client can be connected to more easily (even though it is often ignored).
/// # Examples
/// ```
/// use fuels_test_helpers::setup_test_provider;
///
/// async fn test_provider() -> Result<(), Box<dyn std::error::Error>> {
///   let (_provider, _address) = setup_test_provider(vec![], vec![], None, None).await;
///   Ok(())
/// }
/// ```
pub async fn setup_test_provider(
    coins: Vec<Coin>,
    messages: Vec<Message>,
    node_config: Option<Config>,
    chain_config: Option<ChainConfig>,
) -> (Provider, SocketAddr) {
    let (client, addr, consensus_parameters) =
        setup_test_client(coins, messages, node_config, chain_config).await;
    (Provider::new(client, consensus_parameters), addr)
}

#[cfg(test)]
mod tests {
    use fuels_accounts::{fuel_crypto::fuel_types::AssetId, ViewOnlyAccount};
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

        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await;

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
        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
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
                        CoinType::Message(_) => panic!("Resources contained messages."),
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

        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await;

        assert_eq!(
            wallets.get(31).unwrap().address().to_string(),
            "fuel1rsjlwjzx0px3zu2al05jdlzp4j5quqzlk7pzyk4g45x6m7r3elzsz9dwh4".to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn generated_wallets_with_custom_chain_config() -> Result<()> {
        use fuel_core_chain_config::ChainConfig;
        use fuel_tx::ConsensusParameters;

        let chain_config = ChainConfig {
            transaction_parameters: ConsensusParameters::DEFAULT
                .with_max_gas_per_tx(10_000_000_000),
            block_gas_limit: 10_000_000_000,
            ..ChainConfig::default()
        };

        let wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(4), Some(3), Some(2_000_000_000)),
            None,
            Some(chain_config),
        )
        .await;

        assert_eq!(wallets.len(), 4);

        for wallet in wallets.into_iter() {
            assert_eq!(
                wallet
                    .try_provider()?
                    .client
                    .chain_info()
                    .await?
                    .consensus_parameters
                    .max_gas_per_tx,
                10_000_000_000
            );
            assert_eq!(wallet.get_coins(AssetId::default()).await?.len(), 3);
            assert_eq!(
                wallet
                    .get_balances()
                    .await?
                    .get("0000000000000000000000000000000000000000000000000000000000000000")
                    .expect("Failed to get value"),
                &6_000_000_000
            );
        }

        Ok(())
    }
}
