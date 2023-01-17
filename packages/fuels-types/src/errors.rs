use std::{array::TryFromSliceError, fmt, str::Utf8Error};

use fuel_tx::{CheckError, Receipt};
use strum::ParseError;
use thiserror::Error;

#[derive(Debug)]
pub enum CodecError {
    InvalidData(String),
    Utf8Error(Utf8Error),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<Utf8Error> for CodecError {
    fn from(e: Utf8Error) -> CodecError {
        CodecError::Utf8Error(e)
    }
}

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

impl From<CodecError> for Error {
    fn from(err: CodecError) -> Error {
        match err {
            CodecError::InvalidData(s) => Error::InvalidData(s),
            CodecError::Utf8Error(e) => Error::Utf8Error(e),
        }
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::InvalidType(err.to_string())
    }
}

impl From<proc_macro2::LexError> for Error {
    fn from(err: proc_macro2::LexError) -> Error {
        Error::ParseTokenStreamError(err.to_string())
    }
}

impl From<bech32::Error> for Error {
    fn from(err: bech32::Error) -> Error {
        Error::InvalidData(err.to_string())
    }
}

impl From<TryFromSliceError> for Error {
    fn from(err: TryFromSliceError) -> Error {
        Error::InvalidData(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Error {
        Error::ParseTokenStreamError(err.to_string())
    }
}
