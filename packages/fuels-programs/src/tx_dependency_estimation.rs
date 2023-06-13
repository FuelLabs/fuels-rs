use fuel_tx::Receipt;
use fuels_core::types::errors::{Error, Result};

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

#[derive(Default, Debug)]
pub(crate) struct TxDependencyEstimator {}

impl TxDependencyEstimator {
    pub async fn estimate_tx_dependencies<T: TxDependencyEstimation + Send + Sync>(
        mut program_handler: T,
        max_attempts: Option<u64>,
    ) -> Result<T> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            match program_handler.simulate().await {
                Ok(_) => return Ok(program_handler),

                Err(Error::RevertTransactionError { ref receipts, .. }) => {
                    program_handler = program_handler.append_missing_deps(receipts);
                }

                Err(other_error) => return Err(other_error),
            }
        }

        program_handler.simulate().await.map(|_| program_handler)
    }
}

#[async_trait::async_trait]
pub trait TxDependencyEstimation {
    async fn simulate(&mut self) -> Result<()>;
    fn append_missing_deps(self, receipts: &[Receipt]) -> Self;
}
