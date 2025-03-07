use std::path::Path;

use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    error,
    traits::{AddressResolver, Signer},
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        errors::Result,
    },
};
use rand::{CryptoRng, Rng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/1179993420'/0'/0/0";
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
}

impl AddressResolver for PrivateKeySigner {
    fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locked {
    address: Bech32Address,
}

impl Locked {
    pub fn new(address: Bech32Address) -> Self {
        Self { address }
    }
}

impl AddressResolver for Locked {
    fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeSigner {
    address: Bech32Address,
}

impl From<PrivateKeySigner> for FakeSigner {
    fn from(signer: PrivateKeySigner) -> Self {
        Self {
            address: signer.address().clone(),
        }
    }
}

impl FakeSigner {
    pub fn new(address: Bech32Address) -> Self {
        Self { address }
    }
}

#[async_trait]
impl Signer for FakeSigner {
    async fn sign(&self, _message: Message) -> Result<Signature> {
        Ok(Signature::default())
    }
}

impl AddressResolver for FakeSigner {
    fn address(&self) -> &Bech32Address {
        &self.address
    }
}

#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct KeySaved {
    key: SecretKey,
    #[zeroize(skip)]
    uuid: String,
}

impl KeySaved {
    pub fn key(&self) -> SecretKey {
        self.key
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }
}

/// Creates a new key and stores its encrypted version in the given path.
pub fn new_key_from_keystore<P, R, S>(dir: P, rng: &mut R, password: S) -> Result<KeySaved>
where
    P: AsRef<Path>,
    R: Rng + CryptoRng + CryptoRng,
    S: AsRef<[u8]>,
{
    let (secret, uuid) =
        eth_keystore::new(dir, rng, password, None).map_err(|e| error!(Other, "{e}"))?;

    let key = SecretKey::try_from(secret.as_slice()).expect("should have correct size");

    Ok(KeySaved { key, uuid })
}

/// Recreates a key from an encrypted JSON wallet given the provided path and password.
pub fn load_key_from_keystore<P, S>(keypath: P, password: S) -> Result<SecretKey>
where
    P: AsRef<Path>,
    S: AsRef<[u8]>,
{
    let secret = eth_keystore::decrypt_key(keypath, password).map_err(|e| error!(Other, "{e}"))?;
    let secret_key =
        SecretKey::try_from(secret.as_slice()).expect("Decrypted key should have a correct size");
    Ok(secret_key)
}

// TODO: segfault, this needs to go into a struct along with other keystore stuff

/// Encrypts the private key with the given password and saves it
/// to the given path.
pub fn save_key_to_keystore<P, S, R>(
    key: SecretKey,
    dir: P,
    password: S,
    mut rng: R,
) -> Result<String>
where
    P: AsRef<Path>,
    S: AsRef<[u8]>,
    R: Rng + CryptoRng,
{
    eth_keystore::encrypt_key(dir, &mut rng, *key, password, None).map_err(|e| error!(Other, "{e}"))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rand::{rngs::StdRng, thread_rng, SeedableRng};
    use tempfile::tempdir;

    use crate::signers::PrivateKeySigner;

    use super::*;

    #[tokio::test]
    async fn encrypted_json_keystore() -> Result<()> {
        let dir = tempdir()?;
        let mut rng = rand::thread_rng();

        // Create a key to be stored in the keystore.
        let key_saved = new_key_from_keystore(&dir, &mut rng, "password")?;
        let signer = PrivateKeySigner::new(key_saved.key());

        // sign a message using the above key.
        let message = Message::new("Hello there!".as_bytes());
        let signature = signer.sign(message).await?;

        // Read from the encrypted JSON keystore and decrypt it.
        let path = Path::new(dir.path()).join(key_saved.uuid());
        let recovered_key = load_key_from_keystore(path.clone(), "password")?;
        let signer = PrivateKeySigner::new(recovered_key);

        // Sign the same message as before and assert that the signature is the same.

        let signature2 = signer.sign(message).await?;
        assert_eq!(signature, signature2);

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
        Ok(())
    }

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
    async fn wallet_from_mnemonic_phrase() -> Result<()> {
        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create first key from mnemonic phrase.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/60'/0'/0/0")?;
        let signer = PrivateKeySigner::new(key);

        let expected_plain_address =
            "df9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185";
        let expected_address = "fuel1m7wsumrvtaw6d6pwtcd809627ejzhk69pggvg0cvdyg2yynqqxzseuzply";

        assert_eq!(signer.address().hash().to_string(), expected_plain_address);
        assert_eq!(signer.address().to_string(), expected_address);

        // Create a second key from the same phrase.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/60'/1'/0/0")?;
        let signer = PrivateKeySigner::new(key);

        let expected_second_plain_address =
            "261191b0164a24fd0fd51566ec5e5b0b9ba8fb2d42dc9cf7dbbd6f23d2742759";
        let expected_second_address =
            "fuel1ycgervqkfgj06r74z4nwchjmpwd637edgtwfea7mh4hj85n5yavszjk4cc";

        assert_eq!(
            signer.address().hash().to_string(),
            expected_second_plain_address
        );
        assert_eq!(signer.address().to_string(), expected_second_address);

        Ok(())
    }

    #[tokio::test]
    async fn encrypt_and_store_keys_from_mnemonic() -> Result<()> {
        let dir = tempdir()?;

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create first key from mnemonic phrase.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/60'/0'/0/0")?;

        let uuid = save_key_to_keystore(key, &dir, "password", thread_rng())?;

        let path = Path::new(dir.path()).join(uuid);

        let recovered_key = load_key_from_keystore(&path, "password")?;

        assert_eq!(key, recovered_key);

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
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
