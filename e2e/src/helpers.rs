use std::str::FromStr;

use fuels::{
    accounts::{provider::Provider, wallet::WalletUnlocked},
    core::error,
    crypto::SecretKey,
    test_helpers::{ChainConfig, NodeConfig, WalletsConfig},
    types::errors::Result,
};

use crate::{TESTNET_NODE_URL, TEST_WALLETS_COUNT};

pub fn is_testnet() -> bool {
    option_env!("E2E_TARGET").unwrap_or_default() == "testnet"
}

pub async fn maybe_connect_to_testnet_and_get_wallets(
    wallet_config: WalletsConfig,
    node_config: Option<NodeConfig>,
    chain_config: Option<ChainConfig>,
) -> Result<Vec<WalletUnlocked>> {
    if is_testnet() {
        let num_wallets = wallet_config.num_wallets();
        if num_wallets > TEST_WALLETS_COUNT {
            error!(
                Provider,
                "Can't get more than {} wallets when E2E_TARGET_TESTNET is set", TEST_WALLETS_COUNT
            );
        }

        let provider = Provider::connect(TESTNET_NODE_URL)
            .await
            .unwrap_or_else(|_| panic!("should be able to connect to {TESTNET_NODE_URL}"));
        let wallets = (1..=num_wallets)
            .map(|wallet_counter| {
                let private_key_var_name = format!("TEST_WALLET_SECRET_KEY_{wallet_counter}");
                let private_key_string =
                    std::env::var(&private_key_var_name).unwrap_or_else(|_| {
                        panic!("Should find private key in environment as {private_key_var_name}")
                    });
                let private_key = SecretKey::from_str(private_key_string.as_str())
                    .expect("Should be able to transform into private key");
                WalletUnlocked::new_from_private_key(private_key, Some(provider.clone()))
            })
            .collect::<Vec<_>>();
        Ok(wallets)
    } else {
        fuels::test_helpers::launch_custom_provider_and_get_wallets(
            wallet_config,
            node_config,
            chain_config,
        )
        .await
    }
}

pub async fn maybe_connect_to_testnet_and_get_wallet() -> Result<WalletUnlocked> {
    let mut wallets = maybe_connect_to_testnet_and_get_wallets(
        WalletsConfig::new(Some(1), None, None),
        None,
        None,
    )
    .await?;
    Ok(wallets.pop().expect("should have one wallet"))
}
