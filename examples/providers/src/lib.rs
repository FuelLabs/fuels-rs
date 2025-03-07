#[cfg(test)]
mod tests {
    use std::time::Duration;

    use fuels::prelude::Result;

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
                wallet_signer.address(),
                *consensus_parameters.base_asset_id(),
            )
            .await?;
        assert_eq!(coins.len(), 1);
        // ANCHOR_END: get_coins

        // ANCHOR: get_spendable_resources
        let filter = ResourceFilter {
            from: wallet_signer.address().clone(),
            amount: 1,
            ..Default::default()
        };
        let spendable_resources = provider.get_spendable_resources(filter).await?;
        assert_eq!(spendable_resources.len(), 1);
        // ANCHOR_END: get_spendable_resources

        // ANCHOR: get_balances
        let _balances = provider.get_balances(wallet_signer.address()).await?;
        // ANCHOR_END: get_balances

        Ok(())
    }
}
