use std::{array::TryFromSliceError, str::Utf8Error};

use fuel_tx::{CheckError, Receipt};
use strum::ParseError;
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
    #[error("Parse integer error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Parse boolean error: {0}")]
    ParseBoolError(#[from] std::str::ParseBoolError),
    #[error("Parse hex error: {0}")]
    ParseHexError(#[from] hex::FromHexError),
    #[error("Parse token stream error: {0}")]
    ParseTokenStreamError(String),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
    #[error("Compilation error: {0}")]
    CompilationError(String),
    #[error("Instantiation error: {0}")]
    InstantiationError(String),
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
    #[error("Wallet error: {0}")]
    WalletError(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(#[from] CheckError),
    #[error("Revert transaction error: {}, receipts: {:?}", .0, .1)]
    RevertTransactionError(String, Vec<Receipt>),
}

// This macro can only be used for variants that have a String field
// for example `InvalidData`, `InvalidType`, etc.
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
impl_error_from!(InvalidType, ParseError);
impl_error_from!(ParseTokenStreamError, proc_macro2::LexError);
