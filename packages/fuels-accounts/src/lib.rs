#[cfg(feature = "std")]
mod account;
#[cfg(feature = "std")]
mod accounts_utils;
#[cfg(feature = "std")]
pub mod provider;
#[cfg(feature = "std")]
pub mod wallet;

#[cfg(feature = "std")]
pub use account::*;

#[cfg(feature = "coin-cache")]
mod coin_cache;

pub mod predicate;

use fuels_core::types::errors::{error, Error};
pub(crate) fn try_provider_error() -> Error {
    error!(
        Other,
        "no provider available. Make sure to use `set_provider`"
    )
}
