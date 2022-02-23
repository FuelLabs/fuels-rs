use crate::provider::{Provider, ProviderError};
use crate::signature::Signature;
use crate::Signer;
use async_trait::async_trait;
use fuel_gql_client::client::schema::coin::Coin;
use fuel_tx::crypto::Hasher;
use fuel_tx::{Bytes64, Color, Input, Output, Receipt, Transaction, UtxoId, Witness};
use fuel_types::Address;
use fuel_vm::crypto::secp256k1_sign_compact_recoverable;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::{fmt, io};
use thiserror::Error;

/// A FuelVM-compatible wallet which can be used for signing, sending transactions, and more.
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
/// use fuels_signers::provider::Provider;
/// use fuels_signers::util::test_helpers::setup_local_node;
///
/// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
///   // Generate your secret key
///   let mut rng = StdRng::seed_from_u64(2322u64);
///   let mut secret_seed = [0u8; 32];
///   rng.fill_bytes(&mut secret_seed);
///
///   let secret =
///       SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");
///
///   // Setup local test node
///
///   let provider = Provider::new(setup_local_node(vec![]).await);
///
///   // Create a new local wallet with the newly generated key
///   let wallet = LocalWallet::new_from_private_key(secret, provider)?;
///
///   let message = "my message";
///   let signature = wallet.sign_message(message.as_bytes()).await?;
///
///   // Recover address that signed the message
///   let recovered_address = signature.recover(message).unwrap();
///
///   assert_eq!(wallet.address(), recovered_address);
///
///   // Verify signature
///   signature.verify(message, recovered_address).unwrap();
///   Ok(())
/// }
/// ```
///
/// [`Signature`]: fuels_core::signature::Signature
pub struct Wallet {
    /// The Wallet's private key
    pub(crate) private_key: SecretKey,
    /// The wallet's address. The wallet's address is derived
    /// from the first 32 bytes of SHA-256 hash of the wallet's public key.
    pub(crate) address: Address,

    pub provider: Provider,
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
    pub fn new_from_private_key(
        private_key: SecretKey,
        provider: Provider,
    ) -> Result<Self, WalletError> {
        let secp = Secp256k1::new();

        let public = PublicKey::from_secret_key(&secp, &private_key).serialize_uncompressed();
        let public = Bytes64::try_from(&public[1..])?;
        let hashed = Hasher::hash(public);

        Ok(Self {
            private_key,
            address: Address::new(*hashed),
            provider,
        })
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = provider
    }

    /// Transfer funds from this wallet to another `Address`.
    /// Fails if amount for color is larger than address's spendable coins.
    ///
    /// # Examples
    /// ```
    /// use fuels_signers::provider::Provider;
    /// use fuels_signers::{LocalWallet, Signer};
    /// use fuels_signers::util::test_helpers::{
    ///     setup_address_and_coins, setup_local_node, setup_test_provider,
    /// };
    /// use fuel_tx::{Bytes32, Color, Input, Output, UtxoId};
    /// use rand::{rngs::StdRng, RngCore, SeedableRng};
    /// use secp256k1::SecretKey;
    /// use std::str::FromStr;
    ///
    /// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    ///   // Setup test wallets with 1 coin each
    ///   let (pk_1, mut coins_1) = setup_address_and_coins(1, 1);
    ///   let (pk_2, coins_2) = setup_address_and_coins(1, 1);
    ///   coins_1.extend(coins_2);
    ///
    ///   // Setup a provider and node with both set of coins
    ///   let provider = setup_test_provider(coins_1).await;
    ///
    ///   // Create the actual wallets/signers
    ///   let wallet_1 = LocalWallet::new_from_private_key(pk_1, provider.clone()).unwrap();
    ///   let wallet_2 = LocalWallet::new_from_private_key(pk_2, provider).unwrap();
    ///
    ///   // Transfer 1 from wallet 1 to wallet 2
    ///   let _receipts = wallet_1
    ///        .transfer(&wallet_2.address(), 1, Default::default())
    ///        .await
    ///        .unwrap();
    ///
    ///   let wallet_2_final_coins = wallet_2.get_coins().await.unwrap();
    ///
    ///   // Check that wallet two now has two coins
    ///   assert_eq!(wallet_2_final_coins.len(), 2);
    ///   Ok(())
    /// }
    /// ```
    pub async fn transfer(
        &self,
        to: &Address,
        amount: u64,
        color: Color,
    ) -> io::Result<Vec<Receipt>> {
        let spendable = self.get_spendable_coins(&color, amount).await?;

        let mut inputs: Vec<Input> = vec![];
        let outputs: Vec<Output> = vec![
            Output::coin(*to, amount, color),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its color.
            Output::change(self.address(), 0, color),
        ];

        for coin in spendable {
            let input_coin = Input::coin(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                color,
                0,
                0,
                vec![],
                vec![],
            );

            inputs.push(input_coin);
        }

        // Build transaction and sign it
        let mut tx = self.provider.build_transfer_tx(&inputs, &outputs);
        let _sig = self.sign_transaction(&mut tx).await.unwrap();

        // Note that currently coins being sent aren't marked as spent by the client.
        // This will be coming up soon.
        self.provider.send_transaction(&tx).await.map(Into::into)
    }

    /// Gets coins from this wallet
    /// Note that this is a simple wrapper on provider's `get_coins`.
    pub async fn get_coins(&self) -> Result<Vec<Coin>, WalletError> {
        Ok(self.provider.get_coins(&self.address()).await?)
    }

    /// Gets spendable coins from this wallet.
    /// Note that this is a simple wrapper on provider's
    /// `get_spendable_coins`.
    pub async fn get_spendable_coins(&self, color: &Color, amount: u64) -> io::Result<Vec<Coin>> {
        Ok(self
            .provider
            .get_spendable_coins(&self.address(), *color, amount)
            .await?)
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

    async fn sign_transaction(&self, tx: &mut Transaction) -> Result<Signature, Self::Error> {
        let id = tx.id();
        let sig = secp256k1_sign_compact_recoverable(self.private_key.as_ref(), &*id).unwrap();
        let sig = Signature { compact: sig };

        let witness = vec![Witness::from(sig.compact.as_ref())];

        let mut witnesses: Vec<Witness> = tx.witnesses().to_vec();
        match witnesses.len() {
            0 => tx.set_witnesses(witness),
            _ => {
                witnesses.extend(witness);
                tx.set_witnesses(witnesses)
            }
        }

        Ok(sig)
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
