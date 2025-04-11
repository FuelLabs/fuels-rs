use std::path::{Path, PathBuf};

use fuel_crypto::SecretKey;
use fuels_core::{error, types::errors::Result};
use rand::{CryptoRng, Rng};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct KeySaved {
    key: SecretKey,
    #[zeroize(skip)]
    uuid: String,
}

impl KeySaved {
    pub fn key(&self) -> &SecretKey {
        &self.key
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }
}

/// A Keystore encapsulates operations for key management such as creation, loading,
/// and saving of keys into a specified directory.
pub struct Keystore {
    dir: PathBuf,
}

impl Keystore {
    /// Creates a new Keystore instance with the provided directory.
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Loads and decrypts a key from the keystore using the given UUID and password.
    pub fn load_key<S>(&self, uuid: &str, password: S) -> Result<SecretKey>
    where
        S: AsRef<[u8]>,
    {
        let key_path = self.dir.join(uuid);
        let secret =
            eth_keystore::decrypt_key(key_path, password).map_err(|e| error!(Other, "{e}"))?;
        let secret_key = SecretKey::try_from(secret.as_slice())
            .expect("Decrypted key should have a correct size");
        Ok(secret_key)
    }

    /// Encrypts the provided key with the given password and saves it to the keystore.
    /// Returns the generated UUID for the stored key.
    pub fn save_key<R, S>(&self, key: SecretKey, password: S, mut rng: R) -> Result<String>
    where
        R: Rng + CryptoRng,
        S: AsRef<[u8]>,
    {
        // Note: `*key` is used if SecretKey implements Deref to an inner type.
        eth_keystore::encrypt_key(&self.dir, &mut rng, *key, password, None)
            .map_err(|e| error!(Other, "{e}"))
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;
    use tempfile::tempdir;

    use super::*;
    use crate::signers::private_key::PrivateKeySigner;

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
        let signer2 = PrivateKeySigner::new(key);

        let expected_second_plain_address =
            "261191b0164a24fd0fd51566ec5e5b0b9ba8fb2d42dc9cf7dbbd6f23d2742759";
        let expected_second_address =
            "fuel1ycgervqkfgj06r74z4nwchjmpwd637edgtwfea7mh4hj85n5yavszjk4cc";

        assert_eq!(
            signer2.address().hash().to_string(),
            expected_second_plain_address
        );
        assert_eq!(signer2.address().to_string(), expected_second_address);

        Ok(())
    }

    #[tokio::test]
    async fn encrypt_and_store_keys_from_mnemonic() -> Result<()> {
        let dir = tempdir()?;
        let keystore = Keystore::new(dir.path());
        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create a key from the mnemonic phrase.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/60'/0'/0/0")?;
        let uuid = keystore.save_key(key, "password", thread_rng())?;

        let recovered_key = keystore.load_key(&uuid, "password")?;
        assert_eq!(key, recovered_key);

        // Remove the keystore file.
        let key_path = keystore.dir.join(&uuid);
        assert!(std::fs::remove_file(key_path).is_ok());
        Ok(())
    }
}
