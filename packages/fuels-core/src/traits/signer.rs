use async_trait::async_trait;
use fuel_crypto::{Message, Signature};

use crate::types::{bech32::Bech32Address, errors::Result};
use auto_impl::auto_impl;

/// Trait for signing transactions and messages
///
/// Implement this trait to support different signing modes, e.g. hardware wallet, hosted etc.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[auto_impl(&, Box, Rc, Arc)]
pub trait Signer {
    async fn sign(&self, message: Message) -> Result<Signature>;
    fn address(&self) -> &Bech32Address;
}
