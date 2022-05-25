#[cfg(test)]
mod tests {
    use fuels::prelude::*;

    #[tokio::test]
    async fn wallet_transfer() -> Result<(), Box<dyn std::error::Error>> {
        // Setup 2 test wallets with 1 coin each
        let wallets = launch_provider_and_get_wallets(WalletsConfig {
            num_wallets: 2,
            coins_per_wallet: 1,
            coin_amount: 1,
        })
        .await;

        // Transfer 1 from wallet 1 to wallet 2
        let asset_id = Default::default();
        let _receipts = wallets[0]
            .transfer(&wallets[1].address(), 1, asset_id, TxParameters::default())
            .await
            .unwrap();

        let wallet_2_final_coins = wallets[1].get_coins().await.unwrap();

        // Check that wallet 2 now has 2 coins
        assert_eq!(wallet_2_final_coins.len(), 2);
        Ok(())
    }
}
