use std::net::SocketAddr;

use fuel_core::model::coin::Coin;
use fuel_tx::UtxoId;
use fuels_signers::provider::Provider;
use fuels_signers::LocalWallet;

#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider_and_wallet() -> (Provider, LocalWallet) {
    //  We build only 1 coin with amount TEST_COIN_AMOUNT, empirically determined to be
    //  sufficient right now
    let (pk, coins) = crate::setup_address_and_coins(1, crate::DEFAULT_INITIAL_BALANCE);
    // Setup a provider and node with the given coins
    let (provider, _) = setup_test_provider(coins).await;

    let wallet = LocalWallet::new_from_private_key(pk, provider.clone()).unwrap();
    (provider, wallet)
}

// Setup a test provider with the given coins. We return the SocketAddr so the launched node
// client can be connected to more easily (even though it is often ignored).
#[cfg(feature = "fuels-signers")]
pub async fn setup_test_provider(coins: Vec<(UtxoId, Coin)>) -> (Provider, SocketAddr) {
    let (client, addr) = crate::setup_test_client(coins).await;
    (Provider::new(client), addr)
}
