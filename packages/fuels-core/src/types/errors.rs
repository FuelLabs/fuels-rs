pub mod transaction {
    #[derive(thiserror::Error, Debug, Clone)]
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
            receipts: Vec<fuel_tx::Receipt>,
        },
        #[error(": {0}")]
        Other(String),
    }

    impl Reason {
        pub(crate) fn context(self, context: impl std::fmt::Display) -> Self {
            match self {
                Reason::Builder(msg) => Reason::Builder(format!("{context}: {msg}")),
                Reason::Validation(msg) => Reason::Validation(format!("{context}: {msg}")),
                Reason::SqueezedOut(msg) => Reason::SqueezedOut(format!("{context}: {msg}")),
                Reason::Reverted {
                    reason,
                    revert_id,
                    receipts,
                } => Reason::Reverted {
                    reason: format!("{context}: {reason}"),
                    revert_id,
                    receipts,
                },
                Reason::Other(msg) => Reason::Other(format!("{context}: {msg}")),
            }
        }
    }
}

use crate::sealed::Sealed;
use std::fmt::Display;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("io: {0}")]
    IO(String),
    #[error("codec: {0}")]
    Codec(String),
    #[error("transaction {0}")]
    Transaction(transaction::Reason),
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

impl Error {
    pub(crate) fn context(self, context: impl Display) -> Self {
        match self {
            Error::IO(msg) => Error::IO(format!("{context}: {msg}")),
            Error::Codec(msg) => Error::Codec(format!("{context}: {msg}")),
            Error::Transaction(reason) => Error::Transaction(reason.context(context)),
            Error::Provider(msg) => Error::Provider(format!("{context}: {msg}")),
            Error::Other(msg) => Error::Other(format!("{context}: {msg}")),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Provides `context` and `with_context` to `Result`.
///
/// # Examples
/// ```
/// use fuels_core::types:: errors::{Context, Error, Result};
///
/// let res_with_context: Result<()> =
/// Err(Error::Other("some error".to_owned())).context("some context");
///
/// let res_with_context: Result<()> =
/// Err(Error::Other("some error".to_owned())).with_context(|| "some context");
/// ```
pub trait Context<T>: Sealed {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> Sealed for Result<T> {}

impl<T> Context<T> for Result<T> {
    /// Wrap the error value with additional context
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.context(context))
    }

    /// Wrap the error value with additional context that is evaluated lazily
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.context(context())
    }
}

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

impl From<fuel_vm::checked_transaction::CheckError> for Error {
    fn from(err: fuel_vm::checked_transaction::CheckError) -> Error {
        error_transaction!(Validation, "{err:?}")
    }
}

impl From<fuel_tx::ValidityError> for Error {
    fn from(err: fuel_tx::ValidityError) -> Error {
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
impl_error_from!(Other, hex::FromHexError);
impl_error_from!(Other, std::array::TryFromSliceError);
impl_error_from!(Other, std::str::Utf8Error);
impl_error_from!(Other, fuel_abi_types::error::Error);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_context() {
        {
            let res_with_context: Result<()> =
                Err(error!(Provider, "some error")).context("some context");

            assert_eq!(
                res_with_context.unwrap_err().to_string(),
                "provider: some context: some error",
            );
        }
        {
            let res_with_context: Result<()> =
                Err(error_transaction!(Builder, "some error")).context("some context");

            assert_eq!(
                res_with_context.unwrap_err().to_string(),
                "transaction builder: some context: some error"
            );
        }
    }

    #[test]
    fn result_with_context() {
        {
            let res_with_context: Result<()> =
                Err(error!(Other, "some error")).with_context(|| "some context");

            assert_eq!(
                res_with_context.unwrap_err().to_string(),
                "some context: some error",
            );
        }
        {
            let res_with_context: Result<()> =
                Err(error_transaction!(Validation, "some error")).with_context(|| "some context");

            assert_eq!(
                res_with_context.unwrap_err().to_string(),
                "transaction validation: some context: some error"
            );
        }
    }
}
