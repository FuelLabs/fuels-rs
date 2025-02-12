#[cfg(test)]
mod client;
#[cfg(test)]
mod fuel_node;
#[cfg(test)]
mod kms;

#[cfg(test)]
mod tests {
    use crate::fuel_node::FuelNode;
    use anyhow::Result;
    use crate::kms::Kms;

    #[tokio::test(flavor = "multi_thread")]
    async fn aws_wallet() -> Result<()> {

        let kms = Kms::default().with_show_logs(false).start().await?;
        let key = kms.create_key().await?;
        let fuel_node_process = FuelNode::default().with_show_logs(false).start().await?;
        
        fuel_node_process.fund(key.kms_data.address, 5_000_000_000).await?;



        Ok(())
    }
}
