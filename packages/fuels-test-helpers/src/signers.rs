use std::net::SocketAddr;

#[cfg(feature = "fuel-core-lib")]
use fuel_core::{model::Coin, service::Config};
use fuel_gql_client::fuel_tx::UtxoId;

#[cfg(not(feature = "fuel-core-lib"))]
use fuel_core_interfaces::model::Coin;

#[cfg(not(feature = "fuel-core-lib"))]
use crate::node::Config;

use fuels_signers::{provider::Provider, LocalWallet, Signer};

use crate::{setup_single_asset_coins, setup_test_client, wallets_config::WalletsConfig};

pub async fn launch_provider_and_get_wallet() -> LocalWallet {
    let mut wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::new_single(None, None), None).await;

    wallets.pop().unwrap()
}

pub async fn launch_custom_provider_and_get_wallets(
    wallet_config: WalletsConfig,
    provider_config: Option<Config>,
) -> Vec<LocalWallet> {
    let mut wallets: Vec<LocalWallet> = (1..=wallet_config.num_wallets)
        .map(|_i| LocalWallet::new_random(None))
        .collect();

    let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(wallet_config.num_wallets as usize);
    for wallet in &wallets {
        let coins: Vec<(UtxoId, Coin)> = setup_single_asset_coins(
            wallet.address(),
            Default::default(),
            wallet_config.coins_per_wallet,
            wallet_config.coin_amount,
        );
        all_coins.extend(coins);
    }

    let (provider, _) = setup_test_provider(all_coins, provider_config).await;

    wallets
        .iter_mut()
        .for_each(|wallet| wallet.set_provider(provider.clone()));

    wallets
}

// Setup a test provider with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
pub async fn setup_test_provider(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Option<Config>,
) -> (Provider, SocketAddr) {
    let (client, addr) = setup_test_client(coins, node_config).await;
    (Provider::new(client), addr)
}

#[cfg(test)]
mod tests {
    use crate::{launch_custom_provider_and_get_wallets, WalletsConfig};
    use fuels_types::errors::Error;

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
}
