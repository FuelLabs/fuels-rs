#[cfg(test)]
mod tests {
    use fuels_rs::prelude::*;

    #[tokio::test]
    async fn wallet_transfer() -> Result<(), Box<dyn std::error::Error>> {
        // Setup test wallets with 1 coin each
        let (pk_1, mut coins_1) = setup_address_and_coins(1, 1);
        let (pk_2, coins_2) = setup_address_and_coins(1, 1);
        coins_1.extend(coins_2);

        // Setup a provider and node with both set of coins
        let (provider, _) = setup_test_provider(coins_1).await;

        // Create the actual wallets/signers
        let wallet_1 = LocalWallet::new_from_private_key(pk_1, provider.clone()).unwrap();
        let wallet_2 = LocalWallet::new_from_private_key(pk_2, provider).unwrap();

        // Transfer 1 from wallet 1 to wallet 2
        let asset_id = Default::default();
        let _receipts = wallet_1
            .transfer(&wallet_2.address(), 1, asset_id)
            .await
            .unwrap();

        let wallet_2_final_coins = wallet_2.get_coins().await.unwrap();

        // Check that wallet two now has two coins
        assert_eq!(wallet_2_final_coins.len(), 2);
        Ok(())
    }
}
