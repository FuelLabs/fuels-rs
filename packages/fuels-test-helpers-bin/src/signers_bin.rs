use std::net::SocketAddr;

use fuel_core_interfaces::model::Coin;
use fuel_gql_client::fuel_tx::UtxoId;

use fuels_signers::{provider::Provider, LocalWallet, Signer};

use crate::setup_test_client_bin;
use crate::{setup_single_asset_coins, wallets_config_bin::WalletsConfig, FuelCoreServer};

pub async fn launch_provider_and_get_single_wallet_bin() -> (FuelCoreServer, LocalWallet) {
    let (fuel_core_server, mut wallets) =
        launch_provider_and_get_wallets_bin(WalletsConfig::new_single(None, None)).await;

    (fuel_core_server, wallets.pop().unwrap())
}

pub async fn launch_provider_and_get_wallets_bin(
    config: WalletsConfig,
) -> (FuelCoreServer, Vec<LocalWallet>) {
    let mut wallets: Vec<LocalWallet> = (1..=config.num_wallets)
        .map(|_i| LocalWallet::new_random(None))
        .collect();

    let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(config.num_wallets as usize);
    for wallet in &wallets {
        let coins: Vec<(UtxoId, Coin)> = setup_single_asset_coins(
            wallet.address(),
            Default::default(),
            config.coins_per_wallet,
            config.coin_amount,
        );
        all_coins.extend(coins);
    }

    let (fuel_core_server, provider, _) = setup_test_provider_bin(all_coins).await;

    wallets
        .iter_mut()
        .for_each(|wallet| wallet.set_provider(provider.clone()));

    (fuel_core_server, wallets)
}

pub async fn setup_test_provider_bin(
    coins: Vec<(UtxoId, Coin)>,
) -> (FuelCoreServer, Provider, SocketAddr) {
    let (fuel_core_server, client, addr) = setup_test_client_bin(coins).await;
    (fuel_core_server, Provider::new(client), addr)
}
