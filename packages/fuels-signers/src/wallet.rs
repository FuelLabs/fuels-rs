use crate::provider::{Provider, ProviderError};
use crate::Signer;
use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_gql_client::client::schema::coin::Coin;
use fuel_tx::{Address, AssetId, Input, Output, Receipt, Transaction, UtxoId, Witness};
use fuels_core::errors::Error;
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
/// use fuel_crypto::{Message, SecretKey};
/// use rand::{rngs::StdRng, RngCore, SeedableRng};
/// use fuels::prelude::*;
///
/// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
///   // Generate your secret key
///   let mut rng = StdRng::seed_from_u64(2322u64);
///   let mut secret_seed = [0u8; 32];
///   rng.fill_bytes(&mut secret_seed);
///
///   let secret = unsafe { SecretKey::from_bytes_unchecked(secret_seed) };
///
///   // Setup local test node
///
///   let (provider, _) = setup_test_provider(vec![]).await;
///
///   // Create a new local wallet with the newly generated key
///   let wallet = LocalWallet::new_from_private_key(secret, provider);
///
///   let message = "my message";
///   let signature = wallet.sign_message(message.as_bytes()).await?;
///
///   // Recover address that signed the message
///   let message = Message::new(message);
///   let recovered_address = signature.recover(&message).unwrap();
///
///   assert_eq!(wallet.address().as_ref(), recovered_address.hash().as_ref());
///
///   // Verify signature
///   signature.verify(&recovered_address, &message).unwrap();
///   Ok(())
/// }
/// ```
///
/// [`Signature`]: fuels_core::signature::Signature
#[derive(Clone)]
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

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::WalletError(e.to_string())
    }
}

impl Wallet {
    pub fn new_from_private_key(private_key: SecretKey, provider: Provider) -> Self {
        let public = PublicKey::from(&private_key);
        let hashed = public.hash();

        Self {
            private_key,
            address: Address::new(*hashed),
            provider,
        }
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = provider
    }

    /// Transfer funds from this wallet to another `Address`.
    /// Fails if amount for asset ID is larger than address's spendable coins.
    ///
    /// # Examples
    /// ```
    /// use fuels::prelude::*;
    /// use fuel_tx::{Bytes32, AssetId, Input, Output, UtxoId};
    /// use rand::{rngs::StdRng, RngCore, SeedableRng};
    /// use std::str::FromStr;
    ///
    /// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    ///   // Setup test wallets with 1 coin each
    ///   let (pk_1, mut coins_1) = setup_address_and_coins(1, 1);
    ///   let (pk_2, coins_2) = setup_address_and_coins(1, 1);
    ///   coins_1.extend(coins_2);
    ///
    ///   // Setup a provider and node with both set of coins
    ///   let (provider, _) = setup_test_provider(coins_1).await;
    ///
    ///   // Create the actual wallets/signers
    ///   let wallet_1 = LocalWallet::new_from_private_key(pk_1, provider.clone());
    ///   let wallet_2 = LocalWallet::new_from_private_key(pk_2, provider);
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
        asset_id: AssetId,
    ) -> Result<Vec<Receipt>, WalletError> {
        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;
        let outputs: Vec<Output> = vec![
            Output::coin(*to, amount, asset_id),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            Output::change(self.address(), 0, asset_id),
        ];

        // Build transaction and sign it
        let mut tx = self.provider.build_transfer_tx(&inputs, &outputs);
        let _sig = self.sign_transaction(&mut tx).await.unwrap();

        Ok(self.provider.send_transaction(&tx).await?)
    }

    /// Returns a proper vector of `Input::Coin`s for the given asset ID, amount, and witness index.
    /// The `witness_index` is the position of the witness
    /// (signature) in the transaction's list of witnesses.
    /// Meaning that, in the validation process, the node will
    /// use the witness at this index to validate the coins returned
    /// by this method.
    pub async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        witness_index: u8,
    ) -> Result<Vec<Input>, WalletError> {
        let spendable = self.get_spendable_coins(&asset_id, amount).await?;
        let mut inputs = vec![];
        for coin in spendable {
            let input_coin = Input::coin(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                asset_id,
                witness_index,
                0,
                vec![],
                vec![],
            );
            inputs.push(input_coin);
        }
        Ok(inputs)
    }

    /// Gets coins from this wallet
    /// Note that this is a simple wrapper on provider's `get_coins`.
    pub async fn get_coins(&self) -> Result<Vec<Coin>, WalletError> {
        Ok(self.provider.get_coins(&self.address()).await?)
    }

    /// Gets spendable coins from this wallet.
    /// Note that this is a simple wrapper on provider's
    /// `get_spendable_coins`.
    pub async fn get_spendable_coins(
        &self,
        asset_id: &AssetId,
        amount: u64,
    ) -> io::Result<Vec<Coin>> {
        self.provider
            .get_spendable_coins(&self.address(), *asset_id, amount)
            .await
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
        let message = Message::new(message);
        let sig = Signature::sign(&self.private_key, &message);
        Ok(sig)
    }

    async fn sign_transaction(&self, tx: &mut Transaction) -> Result<Signature, Self::Error> {
        let id = tx.id();

        // Safety: `Message::from_bytes_unchecked` is unsafe because
        // it can't guarantee that the provided bytes will be the product
        // of a cryptographically secure hash. However, the bytes are
        // coming from `tx.id()`, which already uses `Hasher::hash()`
        // to hash it using a secure hash mechanism.
        let message = unsafe { Message::from_bytes_unchecked(*id) };
        let sig = Signature::sign(&self.private_key, &message);

        let witness = vec![Witness::from(sig.as_ref())];

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
