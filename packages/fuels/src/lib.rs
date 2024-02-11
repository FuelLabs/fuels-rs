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
        field, ConsensusParameters, ContractIdExt, ContractParameters, FeeParameters, GasCosts,
        PredicateParameters, Receipt, ScriptExecutionResult, ScriptParameters, StorageSlot,
        Transaction as FuelTransaction, TxId, TxParameters, TxPointer, UtxoId, Witness,
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

pub mod crypto {
    pub use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
}

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
            Account, ViewOnlyAccount,
        },
        core::{
            codec::{LogDecoder, LogId, LogResult},
            traits::Signer,
        },
        programs::{
            call_utils::TxDependencyExtension,
            contract::{
                CallParameters, Contract, LoadConfiguration, MultiContractCallHandler,
                SettableContract, StorageConfiguration,
            },
        },
        test_helpers::*,
        types::transaction_builders::*,
    };
    pub use super::{
        core::constants::*,
        macros::{abigen, setup_program_test},
        types::{
            bech32::{Bech32Address, Bech32ContractId},
            errors::{Error, Result},
            transaction::*,
            Address, AssetId, Bytes, ContractId, RawSlice, Salt,
        },
    };
}
