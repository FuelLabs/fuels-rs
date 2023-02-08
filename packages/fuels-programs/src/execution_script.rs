use std::fmt::Debug;

use fuel_tx::{FormatValidityChecks, Receipt, Script, ScriptExecutionResult};

use fuels_signers::provider::Provider;
use fuels_types::errors::{Error, Result};

/// [`ExecutableFuelCall`] provides methods to create and call/simulate a transaction that carries
/// out contract method calls or script calls
#[derive(Debug)]
pub struct ExecutableFuelCall {
    pub tx: Script,
}

impl ExecutableFuelCall {
    pub fn new(tx: Script) -> Self {
        Self { tx }
    }

    /// Execute the transaction in a state-modifying manner.
    pub async fn execute(&self, provider: &Provider) -> Result<Vec<Receipt>> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
        )?;

        provider.send_transaction(&self.tx).await
    }

    /// Execute the transaction in a simulated manner, not modifying blockchain state
    pub async fn simulate(&self, provider: &Provider) -> Result<Vec<Receipt>> {
        let chain_info = provider.chain_info().await?;

        self.tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
        )?;

        let receipts = provider.dry_run(&self.tx.clone().into()).await?;
        Self::validate_script_succedded(&receipts)?;

        Ok(receipts)
    }

    fn validate_script_succedded(receipts: &[Receipt]) -> Result<()> {
        receipts
            .iter()
            .find_map(|receipt| match receipt {
                Receipt::ScriptResult { result, .. }
                    if *result != ScriptExecutionResult::Success =>
                {
                    Some(format!("{result:?}"))
                }
                _ => None,
            })
            .map(|error_message| {
                Err(Error::RevertTransactionError {
                    reason: error_message,
                    revert_id: 0,
                    receipts: receipts.to_owned(),
                })
            })
            .unwrap_or(Ok(()))
    }
}
