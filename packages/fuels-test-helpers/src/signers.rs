use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::{model::Coin, service::Config};

use fuel_gql_client::fuel_tx::UtxoId;

#[cfg(not(feature = "fuel-core-lib"))]
use fuel_core_interfaces::model::Coin;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::node::Config;

use fuels_signers::{provider::Provider, LocalWallet, Signer};

use crate::{setup_custom_assets_coins, setup_test_client, wallets_config::*};

/// Launches a local Fuel node, instantiates a provider, and returns a wallet.
/// The provider and the wallets are instantiated with the default configs.
/// For more configurable options, see the `launch_custom_provider_and_get_wallets` function.
/// # Examples
/// ```
/// use fuels_test_helpers::launch_provider_and_get_wallet;
/// use fuels_signers::Signer;
///
/// async fn single_wallet() -> Result<(), Box<dyn std::error::Error>> {
///   let wallet = launch_provider_and_get_wallet().await;
///   dbg!(wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_provider_and_get_wallet() -> LocalWallet {
    let mut wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::new(Some(1), None, None), None).await;

    wallets.pop().unwrap()
}

/// Launches a custom node and provider, along with a configurable number of wallets.
///
/// # Examples
/// ```
/// use fuels_test_helpers::launch_custom_provider_and_get_wallets;
/// use fuels_signers::Signer;
/// use fuels_test_helpers::WalletsConfig;
///
/// async fn multiple_wallets() -> Result<(), Box<dyn std::error::Error>> {
///   let num_wallets = 2;
///   let num_coins = 1;
///   let amount = 1;
///   let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));
///
///   let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
///   let first_wallet = wallets.pop().unwrap();
///   dbg!(first_wallet.address());
///   Ok(())
/// }
/// ```
pub async fn launch_custom_provider_and_get_wallets(
    wallet_config: WalletsConfig,
    provider_config: Option<Config>,
) -> Vec<LocalWallet> {
    let mut wallets: Vec<LocalWallet> = (1..=wallet_config.num_wallets)
        .map(|_i| LocalWallet::new_random(None))
        .collect();

    let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(wallet_config.num_wallets as usize);
    for wallet in &wallets {
        let coins: Vec<(UtxoId, Coin)> =
            setup_custom_assets_coins(wallet.address(), wallet_config.assets.clone());
        all_coins.extend(coins);
    }

    let (provider, _) = setup_test_provider(all_coins, provider_config).await;

    wallets
        .iter_mut()
        .for_each(|wallet| wallet.set_provider(provider.clone()));

    wallets
}

/// Setup a test provider with the given coins. We return the SocketAddr so the launched node
/// client can be connected to more easily (even though it is often ignored).
/// # Examples
/// ```
/// use fuels_test_helpers::setup_test_provider;
///
/// async fn test_provider() -> Result<(), Box<dyn std::error::Error>> {
///   let (_provider, _address) = setup_test_provider(vec![], None).await;
///   Ok(())
/// }
/// ```
pub async fn setup_test_provider(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Option<Config>,
) -> (Provider, SocketAddr) {
    let (client, addr) = setup_test_client(coins, node_config, None).await;
    (Provider::new(client), addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{launch_custom_provider_and_get_wallets, WalletsConfig};
    use fuels_core::constants::BASE_ASSET_ID;
    use fuels_signers::fuel_crypto::fuel_types::AssetId;
    use fuels_types::errors::Error;
    use rand::Fill;

    #[tokio::test]
    async fn test_wallet_config() -> Result<(), Error> {
        let num_wallets = 2;
        let num_coins = 3;
        let amount = 100;
        let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));

        let wallets = launch_custom_provider_and_get_wallets(config, None).await;

        assert_eq!(wallets.len(), num_wallets as usize);

        for wallet in &wallets {
            let coins = wallet.get_coins().await?;

            assert_eq!(coins.len(), num_coins as usize);

            for coin in &coins {
                assert_eq!(coin.amount.0, amount);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_wallet_config_multiple_assets() -> Result<(), Box<dyn std::error::Error>> {
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
        let wallets = launch_custom_provider_and_get_wallets(config, None).await;
        assert_eq!(wallets.len(), num_wallets as usize);

        for asset in assets {
            for wallet in &wallets {
                let coins = wallet
                    .get_spendable_coins(&asset.id, asset.num_coins * asset.coin_amount)
                    .await?;
                assert_eq!(coins.len() as u64, asset.num_coins);

                for coin in coins {
                    assert_eq!(
                        coin.owner.to_string(),
                        format!("0x{}", wallet.address().hash())
                    );
                    assert_eq!(coin.amount.0, asset.coin_amount);
                }
            }
        }
        Ok(())
    }
}
