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
        Transaction as FuelTransaction, TxId, TxParameters, TxPointer, UpgradePurpose,
        UploadSubsection, UtxoId, Witness,
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

pub mod programs {
    pub use fuels_programs::*;
}

pub mod core {
    pub use fuels_core::{codec, constants, offsets, traits, Configurables};
}

pub mod crypto {
    pub use fuel_crypto::{Hasher, Message, PublicKey, SecretKey, Signature};
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

#[doc(hidden)]
pub mod prelude {
    #[cfg(feature = "std")]
    pub use super::{
        accounts::{
            impersonated_account::ImpersonatedAccount,
            predicate::Predicate,
            provider::*,
            wallet::{generate_mnemonic_phrase, WalletUnlocked},
            Account, ViewOnlyAccount,
        },
        core::{
            codec::{LogDecoder, LogId, LogResult},
            traits::Signer,
        },
        macros::setup_program_test,
        programs::{
            calls::{CallHandler, CallParameters, ContractDependency, Execution},
            contract::{Contract, LoadConfiguration, StorageConfiguration},
        },
        test_helpers::*,
        types::transaction_builders::*,
    };
    pub use super::{
        core::constants::*,
        macros::abigen,
        tx::Receipt,
        types::{
            bech32::{Bech32Address, Bech32ContractId},
            errors::{Error, Result},
            transaction::*,
            Address, AssetId, Bytes, ContractId, RawSlice, Salt,
        },
    };
}
