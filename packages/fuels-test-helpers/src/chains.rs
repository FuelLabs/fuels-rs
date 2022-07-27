use crate::utils::retry;
use anyhow::bail;
use fuel_types::AssetId;
use fuels_core::constants::{BASE_ASSET_ID, DEFAULT_SPENDABLE_COIN_AMOUNT};
use fuels_core::tx::{Input, Output, Transaction};
use fuels_signers::provider::Provider;
use fuels_signers::{LocalWallet, Signer};
use fuels_types::errors::Error;
use rand::Rng;
use std::time::Duration;

/// Can be used to produce `amount` number of blocks. A block is produced by
/// submitting a no-op script for execution. Transactions are submitted one
/// after the other. A new transaction won't be submitted until an increase of
/// block height has been noticed.
///
/// # Arguments
///
/// * `wallet`: A `Wallet` that must contain a provider and spendable coins.
/// * `amount`: By how much to increase the block height.
///
/// returns: Result<(), Error>
pub async fn produce_blocks(wallet: &LocalWallet, amount: usize) -> Result<(), Error> {
    let provider = wallet.get_provider()?;
    for _ in 0..amount {
        let height_before_transaction = provider.latest_block_height().await?;

        let transaction = no_op_signed_transaction(wallet).await?;

        provider.send_transaction(&transaction).await?;

        confirm_block_created(provider, height_before_transaction).await?;
    }
    Ok(())
}

async fn confirm_block_created(provider: &Provider, previous_height: u64) -> Result<(), Error> {
    let block_height_increased = || async {
        let current_block_height = provider.latest_block_height().await?;
        if current_block_height > previous_height {
            Ok(())
        } else {
            bail!("There was no increase in block height")
        }
    };

    retry(
        block_height_increased,
        Duration::from_millis(100),
        Duration::from_millis(500),
    )
    .await
    .map_err(|err| {
        Error::InfrastructureError(format!(
            "Couldn't confirm a block generation via no-op script -- {}",
            err
        ))
    })
}

async fn no_op_signed_transaction(wallet: &LocalWallet) -> Result<Transaction, Error> {
    let inputs = wallet
        .get_asset_inputs_for_amount(BASE_ASSET_ID, DEFAULT_SPENDABLE_COIN_AMOUNT, 0)
        .await?;

    let outputs = vec![Output::change(
        wallet.address().into(),
        0,
        AssetId::default(),
    )];

    let mut transaction = generate_no_op_script(inputs, outputs);

    wallet.sign_transaction(&mut transaction).await?;

    Ok(transaction)
}

fn generate_no_op_script(inputs: Vec<Input>, outputs: Vec<Output>) -> Transaction {
    if let Transaction::Script {
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        receipts_root: _,
        script,
        script_data: _,
        inputs: _,
        outputs: _,
        witnesses,
        metadata: _,
    } = Transaction::default()
    {
        let random_data = rand::thread_rng().gen::<[u8; 32]>();
        Transaction::script(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            script,
            random_data.into(),
            inputs,
            outputs,
            witnesses,
        )
    } else {
        panic!("Expected Transaction::default() to return a Transaction::Script");
    }
}
