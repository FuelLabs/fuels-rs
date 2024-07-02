mod call_handler;
mod contract_call;
pub mod receipt_parser;
mod script_call;
pub mod traits;
pub mod utils;

pub use call_handler::*;
pub use contract_call::*;
pub use script_call::*;

/// Used to control simulations/dry-runs
#[derive(Debug, Clone, Default)]
pub enum Execution {
    /// The transaction will be subject to all validations -- the tx fee must be covered, witnesses
    /// and UTXOs must be valid, etc.
    #[default]
    Realistic,
    /// Most validation is disabled. Witnesses are replaced with fake ones, fake base assets are
    /// added if necessary. Useful for fetching state without needing an account with base assets.
    StateReadOnly,
}
