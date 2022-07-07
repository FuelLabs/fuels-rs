//! # Fuel Rust SDK.
//!
//! ## Quickstart: `prelude`
//!
//! A prelude is provided which imports all the important data types and traits for you. Use this when you want to quickly bootstrap a new project.
//!
//! ```no_run
//! # #[allow(unused)]
//! use fuels::prelude::*;
//! ```
//!
//! Note that `fuels_abigen_macro` isn't included in the `fuels` crate because
//! it is a `proc_macro` package.
//!
//! Examples on how you can use the types imported by the prelude can be found in
//! the [main test suite](https://github.com/FuelLabs/fuels-rs/blob/master/fuels-abigen-macro/tests/harness.rs)

pub use fuels_core::tx;

pub mod client {
    pub use fuel_gql_client::client::*;
}

pub mod fuels_abigen {
    pub use fuels_abigen_macro::*;
}

pub mod contract {
    pub use fuels_contract::*;
}

pub mod core {
    pub use fuels_core::*;
}

pub mod signers {
    pub use fuels_signers::*;
}

pub mod types {
    pub use fuels_types::*;
}

pub mod test_helpers {
    pub use fuels_test_helpers::*;
}

/// Easy imports of frequently used
#[doc(hidden)]
pub mod prelude {
    //! The fuels-rs prelude
    //!
    //! The purpose of this module is to alleviate imports of many common types:
    //!
    //! ```
    //! # #![allow(unused_imports)]
    //! use fuels::prelude::*;
    //! ```

    pub use super::contract::contract::{Contract, MultiContractCallHandler};
    pub use super::core::constants::*;
    pub use super::core::parameters::*;
    pub use super::core::tx::{Address, AssetId, ContractId};
    pub use super::core::{Token, Tokenizable};
    pub use super::fuels_abigen::abigen;
    pub use super::signers::provider::*;
    pub use super::signers::{LocalWallet, Signer};
    pub use super::test_helpers::Config;
    pub use super::test_helpers::*;
    pub use super::tx::Salt;
    pub use super::types::errors::Error;
}
