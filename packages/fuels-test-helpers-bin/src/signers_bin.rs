// use std::net::SocketAddr;
//
// use fuel_core_interfaces::model::Coin;
// use fuel_gql_client::fuel_tx::UtxoId;
//
// use fuels_signers::{provider::Provider, LocalWallet, Signer};
//
// use crate::setup_test_client_bin;
// use crate::{setup_single_asset_coins, wallets_config_bin::WalletsConfig};
//
// #[cfg(not(feature = "fuel-core-lib"))]
// pub async fn launch_provider_and_get_single_wallet() -> LocalWallet {
//     let mut wallets =
//         launch_provider_and_get_wallets(WalletsConfig::new_single(None, None)).await;
//
//     wallets.pop().unwrap()
// }
//
// #[cfg(not(feature = "fuel-core-lib"))]
// pub async fn launch_provider_and_get_wallets(
//     config: WalletsConfig,
// ) -> Vec<LocalWallet> {
//     let mut wallets: Vec<LocalWallet> = (1..=config.num_wallets)
//         .map(|_i| LocalWallet::new_random(None))
//         .collect();
//
//     let mut all_coins: Vec<(UtxoId, Coin)> = Vec::with_capacity(config.num_wallets as usize);
//     for wallet in &wallets {
//         let coins: Vec<(UtxoId, Coin)> = setup_single_asset_coins(
//             wallet.address(),
//             Default::default(),
//             config.coins_per_wallet,
//             config.coin_amount,
//         );
//         all_coins.extend(coins);
//     }
//
//     let (provider, _) = setup_test_provider(all_coins).await;
//
//     wallets
//         .iter_mut()
//         .for_each(|wallet| wallet.set_provider(provider.clone()));
//
//     wallets
// }
//
// #[cfg(not(feature = "fuel-core-lib"))]
// pub async fn setup_test_provider(
//     coins: Vec<(UtxoId, Coin)>,
// ) -> (Provider, SocketAddr) {
//     let (client, addr) = setup_test_client_bin(coins).await;
//     (Provider::new(client), addr)
// }
