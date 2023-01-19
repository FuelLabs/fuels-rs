use std::time::Duration;

use fuels_signers::provider::Provider;
use fuels_types::errors::{error, Error};

use crate::utils::retry;

pub async fn confirm_blocks_created(
    provider: &Provider,
    previous_height: u64,
    n_blocks: u64,
) -> Result<(), Error> {
    let block_height_increased = || async {
        let current_block_height = provider.latest_block_height().await?;
        let diff = current_block_height - previous_height;

        if diff == n_blocks {
            return Ok(());
        }

        Err(error!(
            InfrastructureError,
            "The increase in block height is {diff}, expected {n_blocks}"
        ))
    };

    retry(
        block_height_increased,
        Duration::from_millis(100),
        Duration::from_millis(500),
    )
    .await
    .map_err(|err| {
        error!(
            InfrastructureError,
            "Couldn't confirm a {n_blocks} blocks increase with the `produce_blocks` API -- {err}"
        )
    })
}
