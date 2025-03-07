use std::path::Path;

use async_trait::async_trait;
use cynic::serde::Deserialize;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    error,
    traits::Signer,
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        errors::Result,
    },
};
use rand::{distributions::Standard, prelude::Distribution, CryptoRng, Rng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKeySigner {
    private_key: SecretKey,
    #[zeroize(skip)]
    address: Bech32Address,
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
