use crate::{Opcode, Transaction, REG_ONE};
use anyhow::bail;
use fuel_gql_client::client::FuelClient;
use fuels_core::constants::{
    BASE_ASSET_ID, DEFAULT_BYTE_PRICE, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, DEFAULT_MATURITY,
};
use fuels_core::tx::Input;
use fuels_signers::LocalWallet;
use rand::Rng;
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub async fn add_blocks(wallet: &LocalWallet, amount: usize) -> anyhow::Result<()> {
    let provider = wallet.get_provider().unwrap();
    for _ in 0..amount {
        let height_before_transaction = current_block_height(&provider.client).await?;

        let inputs = wallet
            .get_asset_inputs_for_amount(BASE_ASSET_ID, 1, 1)
            .await?;
        println!("{:?}", inputs);
        let transaction = generate_no_op_script(inputs);
        provider.send_transaction(&transaction).await.unwrap();

        if !check_if_block_height_increased(&provider.client, height_before_transaction).await? {
            bail!("Couldn't confirm a block generation via no-op script");
        }
    }
    Ok(())
}

async fn check_if_block_height_increased(
    client: &FuelClient,
    height_to_compare_with: u64,
) -> anyhow::Result<bool> {
    let shared_client = Arc::new(client.clone());

    let has_block_height_increased = || {
        let client = Arc::clone(&shared_client);
        async move {
            let current_block_height = current_block_height(&client).await?;
            Ok(current_block_height > height_to_compare_with)
        }
    };

    let height_increased =
        retry_until(has_block_height_increased, 5, Duration::from_millis(100)).await?;

    Ok(height_increased)
}

async fn retry_until<Fut>(
    condition: impl Fn() -> Fut,
    max_attempts: usize,
    between_attempts: Duration,
) -> anyhow::Result<bool>
where
    Fut: Future<Output = anyhow::Result<bool>>,
{
    for _ in 0..max_attempts {
        if condition().await? {
            return Ok(true);
        }
        tokio::time::sleep(between_attempts).await;
    }
    Ok(false)
}

pub async fn current_block_height(client: &FuelClient) -> io::Result<u64> {
    Ok(client.chain_info().await?.latest_block.height.0)
}

fn generate_no_op_script(inputs: Vec<Input>) -> Transaction {
    let random_data = rand::thread_rng().gen::<[u8; 32]>();
    Transaction::Script {
        gas_price: DEFAULT_GAS_PRICE,
        gas_limit: DEFAULT_GAS_LIMIT,
        byte_price: DEFAULT_BYTE_PRICE,
        maturity: DEFAULT_MATURITY,
        receipts_root: Default::default(),
        script: Opcode::RET(REG_ONE).to_bytes().to_vec(),
        script_data: random_data.to_vec(),
        inputs,
        outputs: vec![],
        witnesses: vec![],
        metadata: None,
    }
}
