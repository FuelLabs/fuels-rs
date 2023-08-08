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
//! the [test suite](https://github.com/FuelLabs/fuels-rs/tree/master/packages/fuels/tests)

pub mod tx {
    pub use fuel_tx::{
        field, Bytes32, ConsensusParameters, ContractIdExt, Receipt, Salt, ScriptExecutionResult,
        StorageSlot, Transaction as FuelTransaction, TxId,
    };
}

#[cfg(feature = "std")]
pub mod client {
    pub use fuel_core_client::client::{
        pagination::{PageDirection, PaginationRequest},
        FuelClient,
    };
}

pub mod macros {
    pub use fuels_macros::*;
}

#[cfg(feature = "std")]
pub mod programs {
    pub use fuels_programs::*;
}

pub mod core {
    pub use fuels_core::{codec, constants, offsets, traits, Configurables};
}

#[cfg(feature = "std")]
pub mod accounts {
    pub use fuels_accounts::*;
}

pub mod types {
    pub use fuels_core::types::*;
}

#[cfg(feature = "std")]
pub mod test_helpers {
    pub use fuels_test_helpers::*;
}

#[cfg(feature = "std")]
pub mod fuel_node {
    #[cfg(feature = "fuel-core-lib")]
    pub use fuel_core::chain_config::ChainConfig;
    #[cfg(feature = "fuel-core-lib")]
    pub use fuel_core::service::{config::Trigger, Config, DbType, FuelService};
    #[cfg(not(feature = "fuel-core-lib"))]
    pub use fuels_test_helpers::node::{ChainConfig, Config, DbType, FuelService, Trigger};
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
    #[cfg(feature = "std")]
    pub use super::{
        accounts::{
            provider::*,
            wallet::{generate_mnemonic_phrase, WalletUnlocked},
            Account, Signer, ViewOnlyAccount,
        },
        fuel_node::*,
        programs::{
            call_utils::TxDependencyExtension,
            contract::{
                CallParameters, Contract, LoadConfiguration, MultiContractCallHandler,
                SettableContract, StorageConfiguration,
            },
            logs::{LogDecoder, LogId, LogResult},
        },
        test_helpers::*,
    };
    pub use super::{
        core::constants::*,
        macros::{abigen, setup_program_test},
        tx::Salt,
        types::{
            bech32::{Bech32Address, Bech32ContractId},
            errors::{Error, Result},
            transaction::*,
            Address, AssetId, Bytes, ContractId, RawSlice,
        },
    };
}
