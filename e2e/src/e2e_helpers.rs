use crate::{
    aws_kms::{AwsKms, AwsKmsProcess},
};

pub async fn start_aws_kms(logs: bool) -> anyhow::Result<AwsKmsProcess> {
    AwsKms::default().with_show_logs(logs).start().await
}

// pub async fn create_and_fund_aws_kms_key(
//     kms: &FuelService,
//     fuel_node: &FuelNodeProcess,
// ) -> anyhow::Result<KmsTestKey> {
//     let amount = 5_000_000_000;
//     let key = kms.create_key().await?;
//     let address = key.kms_key.address().clone();
//     fuel_node.fund(&address, amount).await?;
//
//     Ok(key)
// }
