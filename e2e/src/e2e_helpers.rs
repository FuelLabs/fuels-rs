use crate::{
    fuel_node::{FuelNode, FuelNodeProcess},
    kms::{Kms, KmsKey, KmsProcess},
};

pub async fn start_kms(logs: bool) -> anyhow::Result<KmsProcess> {
    Kms::default().with_show_logs(logs).start().await
}
pub async fn create_and_fund_kms_keys(
    kms: &KmsProcess,
    fuel_node: &FuelNodeProcess,
) -> anyhow::Result<KmsKey> {
    let amount = 5_000_000_000;
    let key = kms.create_key().await?;
    let address = key.kms_data.address.clone();
    fuel_node.fund(address, amount).await?;

    Ok(key)
}
pub async fn start_fuel_node(logs: bool) -> anyhow::Result<FuelNodeProcess> {
    FuelNode::default().with_show_logs(logs).start().await
}
