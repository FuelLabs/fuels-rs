use crate::{setup_coins, setup_test_client, wallets_config::WalletsConfig};
use fuel_core::model::Coin;
use fuel_tx::UtxoId;
use fuels_signers::provider::Provider;
use fuels_signers::{LocalWallet, Signer};
use std::net::SocketAddr;

#[cfg(feature = "fuels-signers")]
pub async fn launch_provider_and_get_wallets(config: WalletsConfig) -> Vec<LocalWallet> {
    let mut wallets: Vec<LocalWallet> = (1..=config.num_wallets)
        .map(|_i| LocalWallet::new(None))
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

    let (provider, _) = setup_test_provider(all_coins).await;

    wallets
        .iter_mut()
        .for_each(|wallet| wallet.set_provider(provider.clone()));

    wallets
}

// Setup a test provider with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider(coins: Vec<(UtxoId, Coin)>) -> (Provider, SocketAddr) {
    let (client, addr) = setup_test_client(coins).await;
    (Provider::new(client), addr)
}
