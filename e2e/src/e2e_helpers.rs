use crate::google_kms::{GoogleKms, GoogleKmsProcess};
use crate::{
    aws_kms::{AwsKms, AwsKmsProcess, KmsTestKey},
    fuel_node::{FuelNode, FuelNodeProcess},
};

pub async fn start_aws_kms(logs: bool) -> anyhow::Result<AwsKmsProcess> {
    AwsKms::default().with_show_logs(logs).start().await
}

pub async fn start_google_kms(logs: bool) -> anyhow::Result<GoogleKmsProcess> {
    GoogleKms::default().with_show_logs(logs).start().await
}

pub async fn create_and_fund_aws_kms_key(
    kms: &AwsKmsProcess,
    fuel_node: &FuelNodeProcess,
) -> anyhow::Result<KmsTestKey> {
    let amount = 5_000_000_000;
    let key = kms.create_key().await?;
    let address = key.kms_key.address().clone();
    fuel_node.fund(address, amount).await?;

    Ok(key)
}
pub async fn start_fuel_node(logs: bool) -> anyhow::Result<FuelNodeProcess> {
    FuelNode::default().with_show_logs(logs).start().await
}

pub async fn create_and_fund_google_kms_key(
    kms: &GoogleKmsProcess,
    fuel_node: &FuelNodeProcess,
) -> anyhow::Result<()> {
    // let amount = 5_000_000_000;
    // let key = kms.create_key().await?;
    // let address = key.kms_key.address().clone();
    // fuel_node.fund(address, amount).await?;

    Ok(())
}
