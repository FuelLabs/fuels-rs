use std::{array::TryFromSliceError, str::Utf8Error};

use fuel_tx::{Receipt, ValidityError};
use fuel_vm::checked_transaction::CheckError;
use hex::FromHexError;
use thiserror::Error;

pub mod transaction {
    use super::*;

    #[derive(Error, Debug, Clone)]
    pub enum Reason {
        #[error("builder: {0}")]
        Builder(String),
        #[error("validation: {0}")]
        Validation(String),
        #[error("squeezedOut: {0}")]
        SqueezedOut(String),
        #[error("reverted: {reason}, receipts: {receipts:?}")]
        Reverted {
            reason: String,
            revert_id: u64,
            receipts: Vec<Receipt>,
        },
        #[error(": {0}")]
        Other(String),
    }
}
use transaction::Reason;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("io: {0}")]
    IO(String),
    #[error("codec: {0}")]
    Codec(String),
    #[error("transaction {0}")]
    Transaction(Reason),
    #[error("provider: {0}")]
    Provider(String),
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// This macro can only be used for `Error` variants that have a `String` field.
/// Those are: `IO`, `Codec`, `Provider`, `Other`.
#[macro_export]
macro_rules! error {
   ($err_variant:ident, $fmt_str: literal $(,$arg: expr)*) => {
    $crate::types::errors::Error::$err_variant(format!($fmt_str,$($arg),*))
   }
}
pub use error;

/// This macro can only be used for `Error::Transaction` variants that have a `String` field.
/// Those are: `Builder`, `Validation`, `SqueezedOut`, `Other`.
#[macro_export]
macro_rules! error_transaction {
   ($err_variant:ident, $fmt_str: literal $(,$arg: expr)*) => {
    $crate::types::errors::Error::Transaction(
        $crate::types::errors::transaction::Reason::$err_variant(format!($fmt_str,$($arg),*)))
   }
}
pub use error_transaction;

impl From<CheckError> for Error {
    fn from(err: CheckError) -> Error {
        error_transaction!(Validation, "{err:?}")
    }
}

impl From<ValidityError> for Error {
    fn from(err: ValidityError) -> Error {
        error_transaction!(Validation, "{err:?}")
    }
}

macro_rules! impl_error_from {
    ($err_variant:ident, $err_type:ty ) => {
        impl From<$err_type> for $crate::types::errors::Error {
            fn from(err: $err_type) -> $crate::types::errors::Error {
                $crate::types::errors::Error::$err_variant(err.to_string())
            }
        }
    };
}

impl_error_from!(Other, &'static str);
impl_error_from!(Other, bech32::Error);
impl_error_from!(Other, fuel_crypto::Error);
impl_error_from!(Other, serde_json::Error);
impl_error_from!(Other, FromHexError);
impl_error_from!(Other, TryFromSliceError);
impl_error_from!(Other, Utf8Error);
