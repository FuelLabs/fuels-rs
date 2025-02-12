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
    use fuel_core_client::client::schema::schema::__fields::GasCosts::tro;
    use fuels::types::U256;
    use crate::kms::Kms;

    #[tokio::test(flavor = "multi_thread")]
    async fn aws_wallet() -> Result<()> {

        let kms = Kms::default().with_show_logs(false).start().await?;
        let key = kms.create_key().await?;
        let fuel_node = FuelNode::default().with_show_logs(false).start().await?;

        fuel_node.fund(key.kms_data.address.into(), U256::from(10)).await?;

        // let g = Url::parse("http://localhost:4000").unwrap();

        // let fuels_provider = Provider::connect(self.url()).await.unwrap();
        // let fuels_provider = Provider::connect(g).await.unwrap();
        //
        // let mut wallet = WalletUnlocked::new_from_private_key(
        //     SecretKey::from_str("0xde97d8624a438121b86a1956544bd72ed68cd69f2c99555b08b1e8c51ffd511c").unwrap(),
        //     None,
        // );
        //
        // wallet.set_provider(fuels_provider.clone());
        //
        // dbg!(fuels_provider.get_balances(wallet.address()).await?);
        // dbg!(fuels_provider.url());


        Ok(())
    }
}
