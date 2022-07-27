use crate::provider::{Provider, ProviderError};
use crate::Signer;
use async_trait::async_trait;
use coins_bip32::{path::DerivationPath, Bip32Error};
use coins_bip39::{English, Mnemonic, MnemonicError};
use elliptic_curve::rand_core;
use eth_keystore::KeystoreError;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_gql_client::{
    client::{schema::coin::Coin, types::TransactionResponse, PaginatedResult, PaginationRequest},
    fuel_tx::{AssetId, Input, Output, Receipt, Transaction, UtxoId, Witness},
};
use fuels_core::parameters::TxParameters;
use fuels_types::bech32::{Bech32Address, FUEL_BECH32_HRP};
use fuels_types::errors::Error;
use rand::{CryptoRng, Rng};
use std::{collections::HashMap, fmt, io, path::Path, str::FromStr};
use thiserror::Error;

const DEFAULT_DERIVATION_PATH_PREFIX: &str = "m/44'/1179993420'/0'/0/";
type W = English;

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
///
/// use fuel_crypto::Message;
/// use fuels::prelude::*;
///
/// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
///   // Setup local test node
///   let (provider, _) = setup_test_provider(vec![], None).await;
///
///   // Create a new local wallet with the newly generated key
///   let wallet = LocalWallet::new_random(Some(provider));
///
///   let message = "my message";
///   let signature = wallet.sign_message(message.as_bytes()).await?;
///
///   // Recover address that signed the message
///   let message = Message::new(message);
///   let recovered_address = signature.recover(&message).unwrap();
///
///   assert_eq!(wallet.address(), recovered_address);
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
    pub(crate) address: Bech32Address,

    pub(crate) provider: Option<Provider>,
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
    /// Keystore error
    #[error(transparent)]
    KeystoreError(#[from] KeystoreError),
    #[error("invalid mnemonic word count (expected 12, 15, 18, 21, 24, found `{0}`")]
    InvalidMnemonicWordCount(usize),
    #[error(transparent)]
    MnemonicError(#[from] MnemonicError),
    #[error(transparent)]
    Bip32Error(#[from] Bip32Error),
}

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::WalletError(e.to_string())
    }
}

impl Wallet {
    pub fn new_random(provider: Option<Provider>) -> Self {
        let mut rng = rand::thread_rng();
        let private_key = SecretKey::random(&mut rng);

        Self::new_from_private_key(private_key, provider)
    }

    pub fn new_from_private_key(private_key: SecretKey, provider: Option<Provider>) -> Self {
        let public = PublicKey::from(&private_key);
        let hashed = public.hash();

        Self {
            private_key,
            address: Bech32Address::new(FUEL_BECH32_HRP, hashed),
            provider,
        }
    }

    pub fn get_provider(&self) -> Result<&Provider, WalletError> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }

    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> std::io::Result<PaginatedResult<TransactionResponse, String>> {
        self.get_provider()
            .unwrap()
            .get_transactions_by_owner(&self.address, request)
            .await
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// The default derivation path is used.
    pub fn new_from_mnemonic_phrase(
        phrase: &str,
        provider: Option<Provider>,
    ) -> Result<Self, WalletError> {
        let path = format!("{}{}", DEFAULT_DERIVATION_PATH_PREFIX, 0);
        Wallet::new_from_mnemonic_phrase_with_path(phrase, provider, &path)
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// It takes a path to a BIP32 derivation path.
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        provider: Option<Provider>,
        path: &str,
    ) -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::<W>::new_from_phrase(phrase)?;

        let path = DerivationPath::from_str(path)?;

        let derived_priv_key = mnemonic.derive_key(path, None)?;
        let key: &coins_bip32::prelude::SigningKey = derived_priv_key.as_ref();
        let secret_key = unsafe { SecretKey::from_slice_unchecked(key.to_bytes().as_ref()) };

        Ok(Self::new_from_private_key(secret_key, provider))
    }

    /// Creates a new wallet and stores its encrypted version in the given path.
    pub fn new_from_keystore<P, R, S>(
        dir: P,
        rng: &mut R,
        password: S,
        provider: Option<Provider>,
    ) -> Result<(Self, String), WalletError>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng + rand_core::CryptoRng,
        S: AsRef<[u8]>,
    {
        let (secret, uuid) = eth_keystore::new(dir, rng, password)?;

        let secret_key = unsafe { SecretKey::from_slice_unchecked(&secret) };

        let wallet = Self::new_from_private_key(secret_key, provider);

        Ok((wallet, uuid))
    }

    /// Generates a random mnemonic phrase given a random number generator and
    /// the number of words to generate, `count`.
    pub fn generate_mnemonic_phrase<R: Rng>(
        rng: &mut R,
        count: usize,
    ) -> Result<String, WalletError> {
        Ok(Mnemonic::<W>::new_with_count(rng, count)?.to_phrase()?)
    }

    /// Encrypts the wallet's private key with the given password and saves it
    /// to the given path.
    pub fn encrypt<P, S>(&self, dir: P, password: S) -> Result<String, WalletError>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let mut rng = rand::thread_rng();

        Ok(eth_keystore::encrypt_key(
            dir,
            &mut rng,
            *self.private_key,
            password,
        )?)
    }

    /// Recreates a wallet from an encrypted JSON wallet given the provided path and password.
    pub fn load_keystore<P, S>(
        keypath: P,
        password: S,
        provider: Option<Provider>,
    ) -> Result<Self, WalletError>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let secret = eth_keystore::decrypt_key(keypath, password)?;
        let secret_key = unsafe { SecretKey::from_slice_unchecked(&secret) };
        Ok(Self::new_from_private_key(secret_key, provider))
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    /// Transfer funds from this wallet to another `Address`.
    /// Fails if amount for asset ID is larger than address's spendable coins.
    /// Returns the transaction ID that was sent and the list of receipts.
    ///
    /// # Examples
    /// ```
    /// use fuels::prelude::*;
    /// use fuels::test_helpers::setup_single_asset_coins;
    /// use fuels::tx::{Bytes32, AssetId, Input, Output, UtxoId};
    /// use std::str::FromStr;
    /// #[cfg(feature = "fuel-core-lib")]
    /// use fuels_test_helpers::Config;
    ///
    /// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    ///  // Create the actual wallets/signers
    ///  let mut wallet_1 = LocalWallet::new_random(None);
    ///  let mut wallet_2 = LocalWallet::new_random(None);
    ///
    ///   // Setup a coin for each wallet
    ///   let mut coins_1 = setup_single_asset_coins(wallet_1.address(),BASE_ASSET_ID, 1, 1);
    ///   let coins_2 = setup_single_asset_coins(wallet_2.address(),BASE_ASSET_ID, 1, 1);
    ///   coins_1.extend(coins_2);
    ///
    ///   // Setup a provider and node with both set of coins
    ///   let (provider, _) = setup_test_provider(coins_1, None).await;
    ///
    ///   // Set provider for wallets
    ///   wallet_1.set_provider(provider.clone());
    ///   wallet_2.set_provider(provider);
    ///
    ///   // Transfer 1 from wallet 1 to wallet 2
    ///   let _receipts = wallet_1
    ///        .transfer(&wallet_2.address(), 1, Default::default(), TxParameters::default())
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
        to: &Bech32Address,
        amount: u64,
        asset_id: AssetId,
        tx_parameters: TxParameters,
    ) -> Result<(String, Vec<Receipt>), WalletError> {
        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;
        let outputs: Vec<Output> = vec![
            Output::coin(to.into(), amount, asset_id),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            Output::change(self.address().into(), 0, asset_id),
        ];

        // Build transaction and sign it
        let mut tx =
            self.get_provider()
                .unwrap()
                .build_transfer_tx(&inputs, &outputs, tx_parameters);
        let _sig = self.sign_transaction(&mut tx).await.unwrap();

        let receipts = self.get_provider().unwrap().send_transaction(&tx).await?;

        Ok((tx.id().to_string(), receipts))
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
            let input_coin = Input::coin_signed(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                asset_id,
                witness_index,
                0,
            );
            inputs.push(input_coin);
        }
        Ok(inputs)
    }

    /// Gets all coins owned by the wallet, *even spent ones*. This returns actual coins (UTXOs).
    pub async fn get_coins(&self) -> Result<Vec<Coin>, WalletError> {
        Ok(self
            .get_provider()
            .unwrap()
            .get_coins(self.address())
            .await?)
    }

    /// Get some spendable coins of asset `asset_id` owned by the wallet that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    pub async fn get_spendable_coins(
        &self,
        asset_id: &AssetId,
        amount: u64,
    ) -> io::Result<Vec<Coin>> {
        self.get_provider()
            .unwrap()
            .get_spendable_coins(self.address(), *asset_id, amount)
            .await
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(&self, asset_id: &AssetId) -> Result<u64, ProviderError> {
        self.get_provider()?
            .get_asset_balance(self.address(), *asset_id)
            .await
    }

    /// Get all the spendable balances of all assets for the wallet. This is different from getting
    /// the coins because we are only returning the sum of UTXOs coins amount and not the UTXOs
    /// coins themselves.
    pub async fn get_balances(&self) -> Result<HashMap<String, u64>, ProviderError> {
        self.get_provider()?.get_balances(self.address()).await
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

    fn address(&self) -> &Bech32Address {
        &self.address
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .finish()
    }
}

#[cfg(test)]
#[cfg(feature = "test-helpers")]
mod tests {
    use super::*;
    use fuel_core::service::{Config, FuelService};
    use fuel_gql_client::client::FuelClient;
    use fuels_types::errors::Error;
    use tempfile::tempdir;

    #[tokio::test]
    async fn encrypted_json_keystore() -> Result<(), Error> {
        let dir = tempdir()?;
        let mut rng = rand::thread_rng();

        let provider = setup().await;

        // Create a wallet to be stored in the keystore.
        let (wallet, uuid) =
            Wallet::new_from_keystore(&dir, &mut rng, "password", Some(provider.clone()))?;

        // sign a message using the above key.
        let message = "Hello there!";
        let signature = wallet.sign_message(message).await?;

        // Read from the encrypted JSON keystore and decrypt it.
        let path = Path::new(dir.path()).join(uuid);
        let recovered_wallet =
            Wallet::load_keystore(&path.clone(), "password", Some(provider.clone()))?;

        // Sign the same message as before and assert that the signature is the same.
        let signature2 = recovered_wallet.sign_message(message).await?;
        assert_eq!(signature, signature2);

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn mnemonic_generation() -> Result<(), Error> {
        let provider = setup().await;

        let mnemonic = Wallet::generate_mnemonic_phrase(&mut rand::thread_rng(), 12)?;

        let _wallet = Wallet::new_from_mnemonic_phrase(&mnemonic, Some(provider))?;
        Ok(())
    }

    #[tokio::test]
    async fn wallet_from_mnemonic_phrase() -> Result<(), Error> {
        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        let provider = setup().await;

        // Create first account from mnemonic phrase.
        let wallet = Wallet::new_from_mnemonic_phrase_with_path(
            phrase,
            Some(provider.clone()),
            "m/44'/60'/0'/0/0",
        )?;

        let expected_plain_address =
            "df9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185";
        let expected_address = "fuel1m7wsumrvtaw6d6pwtcd809627ejzhk69pggvg0cvdyg2yynqqxzseuzply";

        assert_eq!(wallet.address().hash().to_string(), expected_plain_address);
        assert_eq!(wallet.address().to_string(), expected_address);

        // Create a second account from the same phrase.
        let wallet2 =
            Wallet::new_from_mnemonic_phrase_with_path(phrase, Some(provider), "m/44'/60'/1'/0/0")?;

        let expected_second_plain_address =
            "261191b0164a24fd0fd51566ec5e5b0b9ba8fb2d42dc9cf7dbbd6f23d2742759";
        let expected_second_address =
            "fuel1ycgervqkfgj06r74z4nwchjmpwd637edgtwfea7mh4hj85n5yavszjk4cc";

        assert_eq!(
            wallet2.address().hash().to_string(),
            expected_second_plain_address
        );
        assert_eq!(wallet2.address().to_string(), expected_second_address);

        Ok(())
    }

    #[tokio::test]
    async fn encrypt_and_store_wallet_from_mnemonic() -> Result<(), Error> {
        let dir = tempdir()?;

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        let provider = setup().await;

        // Create first account from mnemonic phrase.
        let wallet = Wallet::new_from_mnemonic_phrase_with_path(
            phrase,
            Some(provider.clone()),
            "m/44'/60'/0'/0/0",
        )?;

        let uuid = wallet.encrypt(&dir, "password")?;

        let path = Path::new(dir.path()).join(uuid);

        let recovered_wallet = Wallet::load_keystore(&path, "password", Some(provider))?;

        assert_eq!(wallet.address(), recovered_wallet.address());

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
        Ok(())
    }

    async fn setup() -> Provider {
        let srv = FuelService::new_node(Config::local_node()).await.unwrap();
        let client = FuelClient::from(srv.bound_address);
        Provider::new(client)
    }
}
