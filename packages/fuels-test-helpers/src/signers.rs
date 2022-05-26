use crate::{
    setup_coins, setup_test_client, wallets_config::WalletsConfig, DEFAULT_COIN_AMOUNT,
    DEFAULT_NUM_COINS,
};
use fuel_core::{model::Coin, service::Config};
use fuel_gql_client::fuel_tx::UtxoId;
use fuels_signers::{provider::Provider, LocalWallet, Signer};
use std::net::SocketAddr;

#[cfg(feature = "fuels-signers")]
pub async fn launch_provider_and_get_single_wallet() -> LocalWallet {
    let mut wallets = launch_provider_and_get_wallets(WalletsConfig::new_single(None, None)).await;

    wallets.pop().unwrap()
}

#[cfg(feature = "fuels-signers")]
pub async fn launch_custom_provider_and_get_single_wallet(node_config: Config) -> LocalWallet {
    let mut wallet = LocalWallet::new_random(None);

    let coins: Vec<(UtxoId, Coin)> =
        setup_coins(wallet.address(), DEFAULT_NUM_COINS, DEFAULT_COIN_AMOUNT);

    let (provider, _) = setup_test_provider(coins, node_config).await;

    wallet.set_provider(provider);
    wallet
}

#[cfg(feature = "fuels-signers")]
pub async fn launch_provider_and_get_wallets(config: WalletsConfig) -> Vec<LocalWallet> {
    let mut wallets: Vec<LocalWallet> = (1..=config.num_wallets)
        .map(|_i| LocalWallet::new_random(None))
        .collect();

    let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(config.num_wallets as usize);
    for wallet in &wallets {
        let coins: Vec<(UtxoId, Coin)> = setup_coins(
            wallet.address(),
            config.coins_per_wallet,
            config.coin_amount,
        );
        all_coins.extend(coins);
    }

    let (provider, _) = setup_test_provider(all_coins, Config::local_node()).await;

    wallets
        .iter_mut()
        .for_each(|wallet| wallet.set_provider(provider.clone()));

    wallets
}

// Setup a test provider with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider(
    coins: Vec<(UtxoId, Coin)>,
    node_config: Config,
) -> (Provider, SocketAddr) {
    let (client, addr) = setup_test_client(coins, node_config).await;
    (Provider::new(client), addr)
}

#[cfg(test)]
mod tests {
    use crate::WalletsConfig;

    use super::*;

    #[tokio::test]
    async fn test_wallet_config() {
        let num_wallets = 2;
        let num_coins = 3;
        let amount = 100;
        let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));

        let wallets = launch_provider_and_get_wallets(config).await;

        assert_eq!(wallets.len(), num_wallets as usize);

        for wallet in &wallets {
            let coins = wallet.get_coins().await.unwrap();

            assert_eq!(coins.len(), num_coins as usize);

            for coin in &coins {
                assert_eq!(coin.amount.0, amount);
            }
        }
    }
}
