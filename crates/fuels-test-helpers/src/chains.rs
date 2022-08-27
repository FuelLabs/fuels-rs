use crate::utils::retry;
use anyhow::bail;
use fuels_signers::provider::Provider;
use fuels_types::errors::Error;
use std::time::Duration;

pub async fn confirm_blocks_created(
    provider: &Provider,
    previous_height: u64,
    n_blocks: u64,
) -> Result<(), Error> {
    let block_height_increased = || async {
        let current_block_height = provider.latest_block_height().await?;
        let diff = current_block_height - previous_height;
        if diff == n_blocks {
            Ok(())
        } else {
            bail!("The increase in block height is {diff}, expected {n_blocks}")
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
            "Couldn't confirm a {} blocks increase with the `produce_blocks` API -- {}",
            n_blocks, err
        ))
    })
}
