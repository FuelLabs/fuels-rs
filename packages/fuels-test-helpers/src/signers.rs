use crate::{setup_coins, generate_pk, setup_test_client, wallets_config::WalletsConfig, DEFAULT_INITIAL_BALANCE};
use fuel_core::model::Coin;
use fuel_crypto::SecretKey;
use fuel_tx::UtxoId;
use fuels_signers::provider::Provider;
use fuels_signers::LocalWallet;
use std::net::SocketAddr;

/// Launches a provider and provides a test wallet
#[cfg(feature = "fuels-signers")]
pub async fn launch_provider_and_get_wallet() -> LocalWallet {
    //  We build only 1 coin with amount DEFAULT_INITIAL_BALANCE, empirically determined to be
    //  sufficient right now
    let mut rng = rand::thread_rng();

    let pk = SecretKey::random(&mut rng);
    let coins = setup_coins(&pk, 1, DEFAULT_INITIAL_BALANCE);
    // Setup a provider and node with the given coins
    let (provider, _) = setup_test_provider(coins).await;

    LocalWallet::new_from_private_key(pk, provider)
}

#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider_and_wallets(
    config: WalletsConfig,
) -> (Provider, Vec<LocalWallet>) {
    let pks: Vec<SecretKey> = (1..=config.num_wallets)
        .map(|_i| generate_pk() )
        .collect();

    let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(pks.len());
    for pk in &pks {
        let coins: Vec<(UtxoId, Coin)> =
            setup_coins(pk, config.coins_per_wallet, config.coin_amount);
        all_coins.extend(coins);
    }

    let (provider, _) = setup_test_provider(all_coins).await;

    let wallets: Vec<LocalWallet> = pks
        .iter()
        .map(|pk| LocalWallet::new_from_private_key(*pk, provider.clone()))
        .collect();

    (provider, wallets)
}

// Setup a test provider with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider(coins: Vec<(UtxoId, Coin)>) -> (Provider, SocketAddr) {
    let (client, addr) = setup_test_client(coins).await;
    (Provider::new(client), addr)
}
