#[cfg(test)]
mod tests {
    use std::time::Duration;

    use fuels::{accounts::signers::private_key::PrivateKeySigner, prelude::Result};

    #[ignore = "testnet currently not compatible with the sdk"]
    #[tokio::test]
    async fn connect_to_fuel_node() -> Result<()> {
        // ANCHOR: connect_to_testnet
        use std::str::FromStr;

        use fuels::{crypto::SecretKey, prelude::*};

        // Create a provider pointing to the testnet.
        let provider = Provider::connect("testnet.fuel.network").await.unwrap();

        // Setup a private key
        let secret = SecretKey::from_str(
            "a1447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568",
        )?;

        // Create the wallet
        let wallet = Wallet::new(PrivateKeySigner::new(secret), provider);

        // Get the wallet address. Used later with the faucet
        dbg!(wallet.address().to_string());
        // ANCHOR_END: connect_to_testnet

        let provider = setup_test_provider(vec![], vec![], None, None).await?;
        let port = provider.url().split(':').last().unwrap();

        // ANCHOR: local_node_address
        let _provider = Provider::connect(format!("127.0.0.1:{port}")).await?;
        // ANCHOR_END: local_node_address

        Ok(())
    }
    #[tokio::test]
    async fn query_the_blockchain() -> Result<()> {
        // ANCHOR: setup_test_blockchain
        use fuels::prelude::*;

        // Set up our test blockchain.

        // Create a random signer
        // ANCHOR: setup_single_asset
        let wallet_signer = PrivateKeySigner::random(&mut rand::thread_rng());

        // How many coins in our wallet.
        let number_of_coins = 1;

        // The amount/value in each coin in our wallet.
        let amount_per_coin = 3;

        let coins = setup_single_asset_coins(
            wallet_signer.address(),
            AssetId::zeroed(),
            number_of_coins,
            amount_per_coin,
        );
        // ANCHOR_END: setup_single_asset

        // ANCHOR: configure_retry
        let retry_config = RetryConfig::new(3, Backoff::Fixed(Duration::from_secs(2)))?;
        let provider = setup_test_provider(coins.clone(), vec![], None, None)
            .await?
            .with_retry_config(retry_config);
        // ANCHOR_END: configure_retry
        // ANCHOR_END: setup_test_blockchain

        // ANCHOR: get_coins
        let consensus_parameters = provider.consensus_parameters().await?;
        let coins = provider
            .get_coins(
                &wallet_signer.address(),
                *consensus_parameters.base_asset_id(),
            )
            .await?;
        assert_eq!(coins.len(), 1);
        // ANCHOR_END: get_coins

        // ANCHOR: get_spendable_resources
        let filter = ResourceFilter {
            from: wallet_signer.address(),
            amount: 1,
            ..Default::default()
        };
        let spendable_resources = provider.get_spendable_resources(filter).await?;
        assert_eq!(spendable_resources.len(), 1);
        // ANCHOR_END: get_spendable_resources

        // ANCHOR: get_balances
        let _balances = provider.get_balances(&wallet_signer.address()).await?;
        // ANCHOR_END: get_balances

        Ok(())
    }
}
