extern crate core;

pub mod provider;
pub mod wallet;

use std::error::Error;

use async_trait::async_trait;
#[doc(no_inline)]
pub use fuel_crypto;
use fuel_crypto::Signature;
use fuels_types::{bech32::Bech32Address, transaction::Transaction};
pub use wallet::{Wallet, WalletUnlocked};

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
    async fn sign_transaction<T: Transaction + Send>(
        &self,
        message: &mut T,
    ) -> Result<Signature, Self::Error>;

    /// Returns the signer's Fuel Address
    fn address(&self) -> &Bech32Address;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fuel_crypto::{Message, SecretKey};
    use fuel_tx::{
        Address, AssetId, Bytes32, Input, Output, Transaction as FuelTransaction, TxPointer, UtxoId,
    };
    use fuels_types::transaction::{ScriptTransaction, Transaction};
    use rand::{rngs::StdRng, RngCore, SeedableRng};

    use super::*;
    use crate::wallet::WalletUnlocked;

    #[tokio::test]
    async fn sign_and_verify() -> Result<(), Box<dyn Error>> {
        // ANCHOR: sign_message
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        rng.fill_bytes(&mut secret_seed);

        let secret = unsafe { SecretKey::from_bytes_unchecked(secret_seed) };

        // Create a wallet using the private key created above.
        let wallet = WalletUnlocked::new_from_private_key(secret, None);

        let message = "my message";

        let signature = wallet.sign_message(message).await?;

        // Check if signature is what we expect it to be
        assert_eq!(signature, Signature::from_str("0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d")?);

        // Recover address that signed the message
        let message = Message::new(message);
        let recovered_address = signature.recover(&message)?;

        assert_eq!(wallet.address().hash(), recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;
        Ok(())
        // ANCHOR_END: sign_message
    }

    #[tokio::test]
    async fn sign_tx_and_verify() -> Result<(), Box<dyn Error>> {
        // ANCHOR: sign_tx
        let secret = SecretKey::from_str(
            "5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1",
        )?;
        let wallet = WalletUnlocked::new_from_private_key(secret, None);

        // Set up a dummy transaction.
        let input_coin = Input::coin_signed(
            UtxoId::new(Bytes32::zeroed(), 0),
            Address::from_str(
                "0xf1e92c42b90934aa6372e30bc568a326f6e66a1a0288595e6e3fbd392a4f3e6e",
            )?,
            10000000,
            AssetId::from([0u8; 32]),
            TxPointer::default(),
            0,
            0,
        );

        let output_coin = Output::coin(
            Address::from_str(
                "0xc7862855b418ba8f58878db434b21053a61a2025209889cc115989e8040ff077",
            )?,
            1,
            AssetId::from([0u8; 32]),
        );

        let mut tx: ScriptTransaction = FuelTransaction::script(
            0,
            1000000,
            0,
            hex::decode("24400000")?,
            vec![],
            vec![input_coin],
            vec![output_coin],
            vec![],
        )
        .into();

        // Sign the transaction.
        let signature = wallet.sign_transaction(&mut tx).await?;
        let message = unsafe { Message::from_bytes_unchecked(*tx.id()) };

        // Check if signature is what we expect it to be
        assert_eq!(signature, Signature::from_str("34482a581d1fe01ba84900581f5321a8b7d4ec65c3e7ca0de318ff8fcf45eb2c793c4b99e96400673e24b81b7aa47f042cad658f05a84e2f96f365eb0ce5a511")?);

        // Recover address that signed the transaction
        let recovered_address = signature.recover(&message)?;

        assert_eq!(wallet.address().hash(), recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;
        Ok(())
        // ANCHOR_END: sign_tx
    }
}
