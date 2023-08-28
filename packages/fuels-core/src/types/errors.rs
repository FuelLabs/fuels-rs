use std::{array::TryFromSliceError, str::Utf8Error};

use fuel_tx::{CheckError, Receipt};
use serde::ser::Error as SerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Instantiation error: {0}")]
    InstantiationError(String),
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
    #[error("Account error: {0}")]
    AccountError(String),
    #[error("Wallet error: {0}")]
    WalletError(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(#[from] CheckError),
    #[error("Tried to forward assets to a contract method that is not payable.")]
    AssetsForwardedToNonPayableMethod,
    #[error("Revert transaction error: {reason},\n receipts: {receipts:?}")]
    RevertTransactionError {
        reason: String,
        revert_id: u64,
        receipts: Vec<Receipt>,
    },
    #[error("Transaction build error: {0}")]
    TransactionBuildError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::IOError(err) => Error::IOError(std::io::Error::new(
                std::io::ErrorKind::Other,
                err.to_string(),
            )),
            Error::SerdeJson(err) => Error::SerdeJson(serde_json::Error::custom(err.to_string())),
            Error::Utf8Error(err) => Error::Utf8Error(*err),
            Error::InvalidData(str) => Error::InvalidData(str.clone()),
            Error::InvalidType(str) => Error::InvalidType(str.clone()),
            Error::InstantiationError(str) => Error::InstantiationError(str.clone()),
            Error::InfrastructureError(str) => Error::InfrastructureError(str.clone()),
            Error::AccountError(str) => Error::AccountError(str.clone()),
            Error::WalletError(str) => Error::WalletError(str.clone()),
            Error::ProviderError(str) => Error::ProviderError(str.clone()),
            Error::ValidationError(err) => Error::ValidationError(err.clone()),
            Error::AssetsForwardedToNonPayableMethod => Error::AssetsForwardedToNonPayableMethod,
            Error::RevertTransactionError {
                reason,
                revert_id,
                receipts,
            } => Error::RevertTransactionError {
                reason: reason.clone(),
                revert_id: *revert_id,
                receipts: receipts.clone(),
            },
            Error::TransactionBuildError(str) => Error::TransactionBuildError(str.clone()),
        }
    }
}

/// This macro can only be used for `Error` variants that have a `String` field.
/// Those are: `InvalidData`, `InvalidType`, `InfrastructureError`,
/// `InstantiationError`, `WalletError`, `ProviderError`, `TransactionBuildError`
#[macro_export]
macro_rules! error {
   ($err_variant:ident, $fmt_str: literal $(,$arg: expr)*) => {
       Error::$err_variant(format!($fmt_str,$($arg),*))
   }
}
pub use error;

macro_rules! impl_error_from {
    ($err_variant:ident, $err_type:ty ) => {
        impl From<$err_type> for Error {
            fn from(err: $err_type) -> Error {
                Error::$err_variant(err.to_string())
            }
        }
    };
}

impl_error_from!(InvalidData, bech32::Error);
impl_error_from!(InvalidData, TryFromSliceError);
