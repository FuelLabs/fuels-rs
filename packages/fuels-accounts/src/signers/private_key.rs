use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    traits::Signer,
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        errors::Result,
    },
};
use rand::{CryptoRng, Rng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Generates a random mnemonic phrase given a random number generator and the number of words to
/// generate, `count`.
pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> Result<String> {
    Ok(fuel_crypto::generate_mnemonic_phrase(rng, count)?)
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKeySigner {
    private_key: SecretKey,
    #[zeroize(skip)]
    address: Bech32Address,
}

impl std::fmt::Debug for PrivateKeySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateKeySigner")
            .field("private_key", &"REDACTED")
            .field("address", &self.address)
            .finish()
    }
}

impl PrivateKeySigner {
    pub fn new(private_key: SecretKey) -> Self {
        let public = PublicKey::from(&private_key);
        let hashed = public.hash();
        let address = Bech32Address::new(FUEL_BECH32_HRP, hashed);

        Self {
            private_key,
            address,
        }
    }

    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        Self::new(SecretKey::random(rng))
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[async_trait]
impl Signer for PrivateKeySigner {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let sig = Signature::sign(&self.private_key, &message);

        Ok(sig)
    }

    fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rand::{rngs::StdRng, SeedableRng};

    use crate::signers::derivation::DEFAULT_DERIVATION_PATH;

    use super::*;

    #[tokio::test]
    async fn mnemonic_generation() -> Result<()> {
        let mnemonic = generate_mnemonic_phrase(&mut rand::thread_rng(), 12)?;
        let _wallet = PrivateKeySigner::new(SecretKey::new_from_mnemonic_phrase_with_path(
            &mnemonic,
            DEFAULT_DERIVATION_PATH,
        )?);

        Ok(())
    }

    #[tokio::test]
    async fn sign_and_verify() -> Result<()> {
        // ANCHOR: sign_message
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        rng.fill_bytes(&mut secret_seed);

        let secret = secret_seed.as_slice().try_into()?;

        // Create a signer using the private key created above.
        let signer = PrivateKeySigner::new(secret);

        let message = Message::new("my message".as_bytes());
        let signature = signer.sign(message).await?;

        // Check if signature is what we expect it to be
        assert_eq!(signature, Signature::from_str("0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d")?);

        // Recover address that signed the message
        let recovered_address = signature.recover(&message)?;

        assert_eq!(signer.address().hash(), recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;
        // ANCHOR_END: sign_message

        Ok(())
    }
}
