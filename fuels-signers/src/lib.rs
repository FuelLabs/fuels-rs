mod wallet;

pub use wallet::Wallet;
pub mod signature;

use signature::Signature;

use async_trait::async_trait;
use fuel_tx::Transaction;
use fuel_types::Address;
use std::error::Error;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = Wallet;

/// Trait for signing transactions and messages
///
/// Implement this trait to support different signing modes, e.g. Ledger, hosted etc.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: std::fmt::Debug + Send + Sync {
    type Error: Error + Send + Sync;
    /// Signs the hash of the provided message
    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error>;

    /// Signs the transaction
    async fn sign_transaction(&self, message: &Transaction) -> Result<Signature, Self::Error>;

    /// Returns the signer's Fuel Address
    fn address(&self) -> Address;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use secp256k1::SecretKey;

    use super::*;

    #[tokio::test]
    async fn sign_and_verify() {
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        rng.fill_bytes(&mut secret_seed);

        let secret =
            SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");

        let wallet = LocalWallet::new_from_private_key(secret).unwrap();

        let message = "my message";

        let signature = wallet.sign_message(message.as_bytes()).await.unwrap();

        // Check if signature is what we expect it to be
        assert_eq!(signature.compact, Signature::from_str("0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d").unwrap().compact);

        // Recover address that signed the message
        let recovered_address = signature.recover(message).unwrap();

        assert_eq!(wallet.address, recovered_address);

        // Verify signature
        signature.verify(message, recovered_address).unwrap();
    }
}
