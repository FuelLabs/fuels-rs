use crate::provider::{Provider, ProviderError};
use crate::signature::Signature;
use crate::Signer;
use async_trait::async_trait;
use fuel_gql_client::client::schema::coin::Coin;
use fuel_gql_client::client::{FuelClient, PageDirection, PaginationRequest};
use fuel_tx::crypto::Hasher;
use fuel_tx::{Bytes32, Bytes64, Input, Output, Transaction, UtxoId};
use fuel_types::Address;
use fuel_vm::crypto::secp256k1_sign_compact_recoverable;
use fuel_vm::prelude::Opcode;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::{fmt, io};
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

    pub provider: Option<Provider>,
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
    #[error("No provider was setup: make sure to set_provider in your wallet!")]
    NoProvider,
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),
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
            provider: None,
        })
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    /// Transfer funds from this wallet to `Address`.
    pub async fn transfer(&self, to: &Address, amount: u64, utxo: UtxoId) -> io::Result<Bytes32> {
        self.provider
            .as_ref()
            .unwrap()
            .transfer(&self.address(), to, amount, utxo)
            .await
            .map(Into::into)
    }

    /// Gets coins from this wallet
    pub async fn get_coins(&self) -> Result<Vec<Coin>, WalletError> {
        if let Some(provider) = &self.provider {
            Ok(provider.get_coins(&self.address()).await?)
        } else {
            Err(WalletError::NoProvider)
        }
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
        let id = _tx.id();
        let sig = secp256k1_sign_compact_recoverable(self.private_key.as_ref(), &*id).unwrap();

        Ok(Signature { compact: sig })
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
