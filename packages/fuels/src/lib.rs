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
//! Examples on how you can use the types imported by the prelude can be found in
//! the [main test suite](https://github.com/FuelLabs/fuels-rs/blob/master/fuels/tests/harness.rs)

pub mod tx {
    pub use fuel_tx::*;
}

pub mod client {
    pub use fuel_gql_client::client::*;
}

pub mod macros {
    pub use fuels_macros::*;
}

pub mod programs {
    pub use fuels_programs::*;
}

pub mod core {
    pub use fuels_core::*;
}

pub mod signers {
    pub use fuels_signers::*;
}

pub mod types {
    pub use fuels_types::{core::*, *};
}

pub mod test_helpers {
    pub use fuels_test_helpers::*;
}

pub mod fuel_node {
    #[cfg(feature = "fuel-core-lib")]
    pub use fuel_core::service::{Config, FuelService};
    #[cfg(not(feature = "fuel-core-lib"))]
    pub use fuels_test_helpers::node::{Config, FuelService};
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
    pub use super::{
        core::{constants::*, parameters::*},
        fuel_node::*,
        macros::{abigen, setup_contract_test},
        programs::{
            contract::{Contract, MultiContractCallHandler, SettableContract},
            logs::LogDecoder,
        },
        signers::{provider::*, wallet::generate_mnemonic_phrase, Signer, Wallet, WalletUnlocked},
        test_helpers::*,
        tx::Salt,
        types::{
            bech32::{Bech32Address, Bech32ContractId},
            errors::Error,
            Address, AssetId, ContractId,
        },
    };
}
