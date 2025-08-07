use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    traits::Signer,
    types::{Address, errors::Result},
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
    address: Address,
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
        let address = Address::from(*public.hash());

        Self {
            private_key,
            address,
        }
    }

    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        Self::new(SecretKey::random(rng))
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn secret_key(&self) -> SecretKey {
        self.private_key
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for PrivateKeySigner {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let sig = Signature::sign(&self.private_key, &message);

        Ok(sig)
    }

    fn address(&self) -> Address {
        self.address
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rand::{SeedableRng, rngs::StdRng};

    use super::*;
    use crate::signers::derivation::DEFAULT_DERIVATION_PATH;

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
        assert_eq!(
            signature,
            Signature::from_str(
                "0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d"
            )?
        );

        // Recover the public key that signed the message
        let recovered_pub_key: PublicKey = signature.recover(&message)?;

        assert_eq!(*signer.address, *recovered_pub_key.hash());

        // Verify signature
        signature.verify(&recovered_pub_key, &message)?;
        // ANCHOR_END: sign_message

        Ok(())
    }
}
