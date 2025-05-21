#[cfg(test)]
mod tests {
    use anyhow::Result;
    use e2e::e2e_helpers::start_aws_kms;
    use fuels::{
        accounts::{Account, ViewOnlyAccount, signers::kms::aws::AwsKmsSigner, wallet::Wallet},
        core::traits::Signer,
        prelude::{
            AssetId, Contract, LoadConfiguration, TxPolicies, launch_provider_and_get_wallet,
        },
        types::errors::Context,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn fund_aws_wallet() -> Result<()> {
        let kms = start_aws_kms(false).await?;
        let wallet = launch_provider_and_get_wallet().await?;

        let amount = 500000000;
        let key = kms.create_signer().await?;
        let address = key.address();

        wallet
            .transfer(address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        let your_kms_key_id = key.key_id();
        let provider = wallet.provider().clone();

        let aws_client = kms.client();
        // ANCHOR: use_kms_wallet
        let kms_signer = AwsKmsSigner::new(your_kms_key_id, aws_client).await?;
        let wallet = Wallet::new(kms_signer, provider);
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
        let key = kms.create_signer().await?;
        let address = key.address();

        wallet
            .transfer(address, amount, AssetId::zeroed(), TxPolicies::default())
            .await
            .context("Failed to transfer funds")?;

        let your_kms_key_id = key.key_id();
        let provider = wallet.provider().clone();

        let kms_signer = AwsKmsSigner::new(your_kms_key_id, kms.client()).await?;
        let aws_wallet = Wallet::new(kms_signer, provider);

        Contract::load_from(
            "../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&aws_wallet, TxPolicies::default())
        .await?;

        Ok(())
    }
}
