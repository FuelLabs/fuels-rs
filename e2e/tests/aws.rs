
#[cfg(test)]
mod tests {
    use anyhow::Result;
    use e2e::e2e_helpers::start_aws_kms;
    use fuels::prelude::{launch_custom_provider_and_get_wallets, AssetId, Contract, LoadConfiguration, TxPolicies, WalletsConfig};
    use fuels::types::errors::Context;
    use fuels_accounts::kms::AwsWallet;
    use fuels_accounts::{Account, ViewOnlyAccount};

    #[tokio::test(flavor = "multi_thread")]
    async fn fund_aws_wallet() -> Result<()> {
        let kms = start_aws_kms(false).await?;

        let mut wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(1), None, None),
            None,
            None,
        )
            .await?;
        let wallet = wallets.first_mut().expect("No wallets found");

        let amount = 500000000;
        let key = kms.create_key().await?;
        let address = key.kms_key.address().clone();

        wallet
            .transfer(&address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_ENDPOINT_URL", &key.url);

        let your_kms_key_id = key.id;
        let provider = wallet.provider().expect("No provider found").clone();

        // ANCHOR: use_kms_wallet
        let wallet = AwsWallet::with_kms_key(your_kms_key_id, Some(provider)).await?;
        // ANCHOR_END: use_kms_wallet

        let founded_coins = wallet
            .get_coins(AssetId::zeroed())
            .await?
            .first()
            .expect("No coins found")
            .amount;
        assert_eq!(founded_coins, 500000000);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deploy_contract() -> anyhow::Result<()> {
        let kms = start_aws_kms(false).await?;

        let mut wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(1), None, None),
            None,
            None,
        )
            .await?;
        let wallet = wallets.first_mut().expect("No wallets found");

        let amount = 500000000;
        let key = kms.create_key().await?;
        let address = key.kms_key.address().clone();

        wallet
            .transfer(&address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_ENDPOINT_URL", &key.url);

        let your_kms_key_id = key.id;
        let provider = wallet.provider().expect("No provider found").clone();

        let aws_wallet = AwsWallet::with_kms_key(your_kms_key_id, Some(provider)).await?;


        let founded_coins = aws_wallet
            .get_coins(AssetId::zeroed())
            .await?
            .first()
            .expect("No coins found")
            .amount;
        assert_eq!(founded_coins, 500000000);


        let contract_id = Contract::load_from(
            "../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
            .deploy(&aws_wallet, TxPolicies::default())
            .await?;

        // println!("Contract deployed @ {contract_id}");
        //
        // let founded_coins = wallet
        //     .get_coins(AssetId::zeroed())
        //     .await?
        //     .first()
        //     .expect("No coins found")
        //     .amount;
        // assert_eq!(founded_coins, 499998321);

        Ok(())
    }
}
