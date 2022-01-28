use crate::signature::Signature;
use crate::Signer;
use async_trait::async_trait;
use fuel_tx::crypto::Hasher;
use fuel_tx::{Bytes64, Transaction};
use fuel_types::Address;
use fuel_vm::crypto::secp256k1_sign_compact_recoverable;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::fmt;
use thiserror::Error;

/// A FuelVM-compatible private-public key pair which can be used for signing messages.
///
/// # Examples
///
/// ## Signing and Verifying a message
///
/// The wallet can be used to produce ECDSA [`Signature`] objects, which can be
/// then verified.
///
/// ```
/// use fuels_signers::{LocalWallet, Signer};
/// use secp256k1::SecretKey;
/// use rand::{rngs::StdRng, RngCore, SeedableRng};
///
/// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
/// // Generate your secret key
/// let mut rng = StdRng::seed_from_u64(2322u64);
/// let mut secret_seed = [0u8; 32];
/// rng.fill_bytes(&mut secret_seed);
///
/// let secret =
///     SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");
///
/// // Create a new local wallet with the newly generated key
/// let wallet = LocalWallet::new_from_private_key(secret)?;
///
/// let message = "my message";
/// let signature = wallet.sign_message(message.as_bytes()).await?;
///
/// // Recover address that signed the message
/// let recovered_address = signature.recover(message).unwrap();
///
/// assert_eq!(wallet.address(), recovered_address);
///
/// // Verify signature
/// signature.verify(message, recovered_address).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// [`Signature`]: fuels_core::signature::Signature
pub struct Wallet {
    /// The Wallet's private key
    pub(crate) private_key: SecretKey,
    /// The wallet's address. The wallet's address is derived
    /// from the first 32 bytes of SHA-256 hash of the wallet's public key.
    pub(crate) address: Address,
}

#[derive(Error, Debug)]
/// Error thrown by the Wallet module
pub enum WalletError {
    /// Error propagated from the hex crate.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Error propagated by IO operations
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Error propagated by parsing of a slice
    #[error("Failed to parse slice")]
    Parsing(#[from] std::array::TryFromSliceError),
}

impl Wallet {
    pub fn new_from_private_key(private_key: SecretKey) -> Result<Self, WalletError> {
        let secp = Secp256k1::new();

        let public = PublicKey::from_secret_key(&secp, &private_key).serialize_uncompressed();
        let public = Bytes64::try_from(&public[1..])?;
        let hashed = Hasher::hash(public);

        Ok(Self {
            private_key,
            address: Address::new(*hashed),
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for Wallet {
    type Error = WalletError;

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        let message = message.as_ref();
        let message_hash = Hasher::hash(message);

        let sig =
            secp256k1_sign_compact_recoverable(self.private_key.as_ref(), &*message_hash).unwrap();
        Ok(Signature { compact: sig })
    }

    async fn sign_transaction(&self, _tx: &Transaction) -> Result<Signature, Self::Error> {
        todo!()
    }

    fn address(&self) -> Address {
        self.address
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .finish()
    }
}
