#[cfg(test)]
mod tests {
    use fuels::prelude::Result;

    #[tokio::test]
    async fn connect_to_fuel_node() {
        // ANCHOR: connect_to_testnet
        use std::str::FromStr;

        use fuels::{accounts::fuel_crypto::SecretKey, prelude::*};

        // Create a provider pointing to the testnet.
        // This example will not work as the testnet does not support the new version of fuel-core
        // yet
        let provider = Provider::connect("beta-4.fuel.network").await.unwrap();

        // Setup a private key
        let secret =
            SecretKey::from_str("a1447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568")
                .unwrap();

        // Create the wallet
        let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

        // Get the wallet address. Used later with the faucet
        dbg!(wallet.address().to_string());
        // ANCHOR_END: connect_to_testnet

        let (_, addr) = setup_test_provider(vec![], vec![], None, None).await;
        let port = addr.port();

        // ANCHOR: local_node_address
        let _provider = Provider::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        // ANCHOR_END: local_node_address
    }

    #[tokio::test]
    async fn query_the_blockchain() -> Result<()> {
        // ANCHOR: setup_test_blockchain
        use fuels::prelude::*;

        // Set up our test blockchain.

        // Create a random wallet (more on wallets later).
        // ANCHOR: setup_single_asset
        let wallet = WalletUnlocked::new_random(None);

        // How many coins in our wallet.
        let number_of_coins = 1;

        // The amount/value in each coin in our wallet.
        let amount_per_coin = 3;

        let coins = setup_single_asset_coins(
            wallet.address(),
            BASE_ASSET_ID,
            number_of_coins,
            amount_per_coin,
        );
        // ANCHOR_END: setup_single_asset

        let (provider, _) = setup_test_provider(coins.clone(), vec![], None, None).await;
        // ANCHOR_END: setup_test_blockchain

        // ANCHOR: get_coins
        let coins = provider.get_coins(wallet.address(), BASE_ASSET_ID).await?;
        assert_eq!(coins.len(), 1);
        // ANCHOR_END: get_coins

        // ANCHOR: get_spendable_resources
        let filter = ResourceFilter {
            from: wallet.address().clone(),
            amount: 1,
            ..Default::default()
        };
        let spendable_resources = provider.get_spendable_resources(filter).await?;
        assert_eq!(spendable_resources.len(), 1);
        // ANCHOR_END: get_spendable_resources

        // ANCHOR: get_balances
        let _balances = provider.get_balances(wallet.address()).await?;
        // ANCHOR_END: get_balances

        Ok(())
    }
}
