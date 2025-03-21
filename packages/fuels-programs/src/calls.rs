mod call_handler;
mod contract_call;
pub mod receipt_parser;
mod script_call;
pub mod traits;
pub mod utils;

pub use call_handler::*;
pub use contract_call::*;
use fuel_types::BlockHeight;
pub use script_call::*;

/// Used to control simulations/dry-runs
#[derive(Debug, Clone)]
pub struct Execution {
    execution_type: ExecutionType,
    at_height: Option<BlockHeight>,
}

impl Execution {
    /// The transaction will be subject to all validations.
    /// The tx fee must be covered, witnesses and UTXOs must be valid, etc.
    pub fn realistic() -> Self {
        Self {
            execution_type: ExecutionType::Realistic,
            at_height: None,
        }
    }
    /// Most validation is disabled. Witnesses are replaced with fake ones, fake base assets are
    /// added if necessary. Useful for fetching state without needing an account with base assets.
    pub fn state_read_only() -> Self {
        Self {
            execution_type: ExecutionType::StateReadOnly,
            at_height: None,
        }
    }

    /// Simulating at as specific block height is only available if the node is using
    /// `rocksdb` and has been started with the `historical_execution` flag.
    pub fn at_height(mut self, height: impl Into<BlockHeight>) -> Self {
        self.at_height = Some(height.into());
        self
    }
}

impl Default for Execution {
    fn default() -> Self {
        Self::realistic()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ExecutionType {
    Realistic,
    StateReadOnly,
}
