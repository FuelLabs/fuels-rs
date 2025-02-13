#[cfg(test)]
mod aws_kms;
#[cfg(test)]
mod client;
#[cfg(test)]
mod e2e_helpers;
#[cfg(test)]
mod fuel_node;
#[cfg(test)]
mod google_kms;

#[cfg(test)]
mod tests {
    use crate::e2e_helpers::{
        create_and_fund_aws_kms_key, create_and_fund_google_kms_key, start_aws_kms,
        start_fuel_node, start_google_kms,
    };
    use anyhow::Result;
    use fuels::prelude::{AssetId, Provider};
    use fuels_accounts::kms::AwsWallet;
    use fuels_accounts::ViewOnlyAccount;
    use google_cloud_kms::client::{Client, ClientConfig};
    use std::str::FromStr;

    #[tokio::test(flavor = "multi_thread")]
    async fn fund_aws_wallet() -> Result<()> {
        let kms = start_aws_kms(false).await?;
        let fuel_node = start_fuel_node(false).await?;
        let kms_key = create_and_fund_aws_kms_key(&kms, &fuel_node).await?;

        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_ENDPOINT_URL", &kms_key.url);

        let asset_id =
            AssetId::from_str("f8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07")
                .expect("AssetId to be well formed");

        let provider = Provider::connect(fuel_node.url()).await?;
        let wallet = AwsWallet::with_kms_key(kms_key.id, Some(provider)).await?;

        let founded_coins = wallet
            .get_coins(asset_id)
            .await?
            .first()
            .expect("No coins found")
            .amount;
        assert_eq!(founded_coins, 5000000000);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deploy_contract() -> anyhow::Result<()> {
        use fuels::prelude::*;

        let kms = start_aws_kms(false).await?;
        let fuel_node = start_fuel_node(false).await?;
        let kms_key = create_and_fund_aws_kms_key(&kms, &fuel_node).await?;

        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_ENDPOINT_URL", &kms_key.url);

        let asset_id =
            AssetId::from_str("f8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07")
                .expect("AssetId to be well formed");

        let provider = Provider::connect(fuel_node.url()).await?;
        let wallet = AwsWallet::with_kms_key(kms_key.id, Some(provider)).await?;

        let founded_coins = wallet
            .get_coins(asset_id)
            .await?
            .first()
            .expect("No coins found")
            .amount;
        assert_eq!(founded_coins, 5000000000);

        let contract_id = Contract::load_from(
            "../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxPolicies::default())
        .await?;

        println!("Contract deployed @ {contract_id}");

        let founded_coins = wallet
            .get_coins(asset_id)
            .await?
            .first()
            .expect("No coins found")
            .amount;
        assert_eq!(founded_coins, 4999983198);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fund_google_wallet() -> anyhow::Result<()> {
        let mut kms = start_google_kms(true).await?;
        let fuel_node = start_fuel_node(false).await?;
        let a = kms.create_key().await?;
        // dbg!(a.name);
        // let kms_key = create_and_fund_google_kms_key(&kms, &fuel_node).await?;
        // let a = kms.create_key().await?;
        // dbg!(a.name);
        Ok(())
    }
}
