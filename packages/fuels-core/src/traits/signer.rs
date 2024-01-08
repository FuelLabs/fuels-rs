use async_trait::async_trait;
use fuel_crypto::{Message, Signature};

use crate::types::{bech32::Bech32Address, errors::Result};

/// Trait for signing transactions and messages
///
/// Implement this trait to support different signing modes, e.g. Ledger, hosted etc.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: std::fmt::Debug + Send + Sync + 'static {
    async fn sign(&self, message: Message) -> Result<Signature>;
    fn address(&self) -> &Bech32Address;
}
