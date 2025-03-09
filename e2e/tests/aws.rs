#[cfg(test)]
mod tests {
    use anyhow::Result;
    use e2e::e2e_helpers::start_aws_kms;
    use fuels::accounts::kms::{AwsKmsSigner, KmsWallet};
    use fuels::accounts::{Account, ViewOnlyAccount};
    use fuels::prelude::{
        launch_provider_and_get_wallet, AssetId, Contract, LoadConfiguration, Signer, TxPolicies,
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
        let signer = AwsKmsSigner::new(your_kms_key_id, kms.client()).await?;
        let kms_wallet = KmsWallet::new(signer, Some(provider));
        // ANCHOR_END: use_kms_wallet

        let total_base_balance = kms_wallet.get_asset_balance(&AssetId::zeroed()).await?;
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

        let signer = AwsKmsSigner::new(your_kms_key_id, kms.client()).await?;
        let kms_wallet = KmsWallet::new(signer, Some(provider));

        Contract::load_from(
            "../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&kms_wallet, TxPolicies::default())
        .await?;

        Ok(())
    }
}
