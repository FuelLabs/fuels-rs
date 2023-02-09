//! Defines a set of serializable types required for the Fuel VM ABI.
//!
//! We declare these in a dedicated, minimal crate in order to allow for downstream projects to
//! consume or generate these ABI-compatible types without needing to pull in the rest of the SDK.

pub mod bech32;
pub mod block;
pub mod chain_info;
pub mod coin;
pub mod constants;
mod core;
pub mod enum_variants;
pub mod errors;
pub mod message;
pub mod message_proof;
pub mod node_info;
pub mod param_types;
pub mod parameters;
pub mod resource;
pub mod traits;
pub mod transaction;
pub mod transaction_response;

pub use fuel_tx::{Address, AssetId, ContractId};

pub use crate::core::*;
