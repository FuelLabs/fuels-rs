#[cfg(test)]
mod tests {
    use anyhow::Result;
    use e2e::e2e_helpers::start_aws_kms;
    use fuels::accounts::kms::AwsWallet;
    use fuels::accounts::{Account, ViewOnlyAccount};
    use fuels::prelude::{
        launch_provider_and_get_wallet, AssetId, Contract, LoadConfiguration, TxPolicies,
    };
    use fuels::types::errors::Context;

    #[tokio::test(flavor = "multi_thread")]
    async fn fund_aws_wallet() -> Result<()> {
        let kms = start_aws_kms(false).await?;
        let wallet = launch_provider_and_get_wallet().await?;

        let amount = 500000000;
        let key = kms.create_key().await?;
        let address = key.kms_key.address().clone();

        wallet
            .transfer(&address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        let your_kms_key_id = key.id;
        let provider = wallet.provider().expect("No provider found").clone();

        // ANCHOR: use_kms_wallet
        let wallet = AwsWallet::with_kms_key(your_kms_key_id, kms.client(), Some(provider)).await?;
        // ANCHOR_END: use_kms_wallet

        let total_base_balance = wallet.get_asset_balance(&AssetId::zeroed()).await?;
        assert_eq!(total_base_balance, amount);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deploy_contract() -> Result<()> {
        let kms = start_aws_kms(false).await?;

        let wallet = launch_provider_and_get_wallet().await?;

        let amount = 500000000;
        let key = kms.create_key().await?;
        let address = key.kms_key.address().clone();

        wallet
            .transfer(&address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        let your_kms_key_id = key.id;
        let provider = wallet.provider().expect("No provider found").clone();

        let aws_wallet =
            &AwsWallet::with_kms_key(your_kms_key_id, kms.client(), Some(provider)).await?;

        Contract::load_from(
            "../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(aws_wallet, TxPolicies::default())
        .await?;

        Ok(())
    }
}
