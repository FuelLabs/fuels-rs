use std::{fmt, ops, path::Path};

use async_trait::async_trait;
use elliptic_curve::rand_core;
use eth_keystore::KeystoreError;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_tx::{AssetId, Witness};
use fuels_types::{
    bech32::{Bech32Address, FUEL_BECH32_HRP},
    constants::BASE_ASSET_ID,
    errors::{Error, Result},
    input::Input,
    transaction::Transaction,
    transaction_builders::TransactionBuilder,
};
use rand::{CryptoRng, Rng};
use thiserror::Error;

use crate::{
    accounts_utils::{adjust_inputs, adjust_outputs, calculate_base_amount_with_fee},
    provider::{Provider, ProviderError},
    Account, AccountError, AccountResult, Signer, ViewOnlyAccount,
};

pub const DEFAULT_DERIVATION_PATH_PREFIX: &str = "m/44'/1179993420'";

#[derive(Error, Debug)]
/// Error thrown by the Wallet module
pub enum WalletError {
    /// Error propagated from the hex crate.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Error propagated by parsing of a slice
    #[error("Failed to parse slice")]
    Parsing(#[from] std::array::TryFromSliceError),
    /// Keystore error
    #[error(transparent)]
    KeystoreError(#[from] KeystoreError),
    #[error(transparent)]
    FuelCrypto(#[from] fuel_crypto::Error),
    #[error(transparent)]
    ProviderError(#[from] ProviderError),
    #[error("Called `try_provider` method on wallet where no provider was set up")]
    NoProviderError,
}

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::WalletError(e.to_string())
    }
}

type WalletResult<T> = std::result::Result<T, WalletError>;

/// A FuelVM-compatible wallet that can be used to list assets, balances and more.
///
/// Note that instances of the `Wallet` type only know their public address, and as a result can
/// only perform read-only operations.
///
/// In order to sign messages or send transactions, a `Wallet` must first call [`Wallet::unlock`]
/// with a valid private key to produce a [`WalletUnlocked`].
#[derive(Clone)]
pub struct Wallet {
    /// The wallet's address. The wallet's address is derived
    /// from the first 32 bytes of SHA-256 hash of the wallet's public key.
    pub(crate) address: Bech32Address,
    provider: Option<Provider>,
}

/// A `WalletUnlocked` is equivalent to a [`Wallet`] whose private key is known and stored
/// alongside in-memory. Knowing the private key allows a `WalletUlocked` to sign operations, send
/// transactions, and more.
#[derive(Clone, Debug)]
pub struct WalletUnlocked {
    wallet: Wallet,
    pub(crate) private_key: SecretKey,
}

impl Wallet {
    /// Construct a Wallet from its given public address.
    pub fn from_address(address: Bech32Address, provider: Option<Provider>) -> Self {
        Self { address, provider }
    }

    pub fn provider(&self) -> Option<&Provider> {
        self.provider.as_ref()
    }

    pub fn set_provider(&mut self, provider: Provider) -> &mut Self {
        self.provider = Some(provider);
        self
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    /// Unlock the wallet with the given `private_key`.
    ///
    /// The private key will be stored in memory until `wallet.lock()` is called or until the
    /// wallet is `drop`ped.
    pub fn unlock(self, private_key: SecretKey) -> WalletUnlocked {
        WalletUnlocked {
            wallet: self,
            private_key,
        }
    }
}

impl ViewOnlyAccount for Wallet {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::no_provider())
    }

    fn set_provider(&mut self, provider: Provider) -> &mut Wallet {
        self.set_provider(provider)
    }
}

impl WalletUnlocked {
    /// Lock the wallet by `drop`ping the private key from memory.
    pub fn lock(self) -> Wallet {
        self.wallet
    }

    // NOTE: Rather than providing a `DerefMut` implementation, we wrap the `set_provider` method
    // directly. This is because we should not allow the user a `&mut` handle to the inner `Wallet`
    // as this could lead to ending up with a `WalletUnlocked` in an inconsistent state (e.g. the
    // private key doesn't match the inner wallet's public key).
    pub fn set_provider(&mut self, provider: Provider) -> &mut Wallet {
        self.wallet.set_provider(provider)
    }

    /// Creates a new wallet with a random private key.
    pub fn new_random(provider: Option<Provider>) -> Self {
        let mut rng = rand::thread_rng();
        let private_key = SecretKey::random(&mut rng);
        Self::new_from_private_key(private_key, provider)
    }

    /// Creates a new wallet from the given private key.
    pub fn new_from_private_key(private_key: SecretKey, provider: Option<Provider>) -> Self {
        let public = PublicKey::from(&private_key);
        let hashed = public.hash();
        let address = Bech32Address::new(FUEL_BECH32_HRP, hashed);
        Wallet::from_address(address, provider).unlock(private_key)
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// The default derivation path is used.
    pub fn new_from_mnemonic_phrase(
        phrase: &str,
        provider: Option<Provider>,
    ) -> WalletResult<Self> {
        let path = format!("{DEFAULT_DERIVATION_PATH_PREFIX}/0'/0/0");
        Self::new_from_mnemonic_phrase_with_path(phrase, provider, &path)
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// It takes a path to a BIP32 derivation path.
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        provider: Option<Provider>,
        path: &str,
    ) -> WalletResult<Self> {
        let secret_key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, path)?;

        Ok(Self::new_from_private_key(secret_key, provider))
    }

    /// Creates a new wallet and stores its encrypted version in the given path.
    pub fn new_from_keystore<P, R, S>(
        dir: P,
        rng: &mut R,
        password: S,
        provider: Option<Provider>,
    ) -> WalletResult<(Self, String)>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng + rand_core::CryptoRng,
        S: AsRef<[u8]>,
    {
        let (secret, uuid) = eth_keystore::new(dir, rng, password, None)?;

        let secret_key =
            SecretKey::try_from(secret.as_slice()).expect("A new secret should be correct size");

        let wallet = Self::new_from_private_key(secret_key, provider);

        Ok((wallet, uuid))
    }

    /// Encrypts the wallet's private key with the given password and saves it
    /// to the given path.
    pub fn encrypt<P, S>(&self, dir: P, password: S) -> WalletResult<String>
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
            None,
        )?)
    }

    /// Recreates a wallet from an encrypted JSON wallet given the provided path and password.
    pub fn load_keystore<P, S>(
        keypath: P,
        password: S,
        provider: Option<Provider>,
    ) -> WalletResult<Self>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let secret = eth_keystore::decrypt_key(keypath, password)?;
        let secret_key = SecretKey::try_from(secret.as_slice())
            .expect("Decrypted key should have a correct size");
        Ok(Self::new_from_private_key(secret_key, provider))
    }
}

impl ViewOnlyAccount for WalletUnlocked {
    fn address(&self) -> &Bech32Address {
        self.wallet.address()
    }

    fn try_provider(&self) -> AccountResult<&Provider> {
        self.provider.as_ref().ok_or(AccountError::no_provider())
    }

    fn set_provider(&mut self, provider: Provider) -> &mut Self {
        self.wallet.set_provider(provider);
        self
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Account for WalletUnlocked {
    /// Returns a vector consisting of `Input::Coin`s and `Input::Message`s for the given
    /// asset ID and amount. The `witness_index` is the position of the witness (signature)
    /// in the transaction's list of witnesses. In the validation process, the node will
    /// use the witness at this index to validate the coins returned by this method.
    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        witness_index: Option<u8>,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount)
            .await?
            .into_iter()
            .map(|resource| Input::resource_signed(resource, witness_index.unwrap_or_default()))
            .collect::<Vec<Input>>())
    }

    async fn add_fee_resources<Tb: TransactionBuilder>(
        &self,
        mut tb: Tb,
        previous_base_amount: u64,
        witness_index: Option<u8>,
    ) -> Result<Tb::TxType> {
        let consensus_parameters = self
            .try_provider()?
            .chain_info()
            .await?
            .consensus_parameters;
        tb = tb.set_consensus_parameters(consensus_parameters);

        let new_base_amount =
            calculate_base_amount_with_fee(&tb, &consensus_parameters, previous_base_amount);

        let new_base_inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, new_base_amount, witness_index)
            .await?;

        adjust_inputs(&mut tb, new_base_inputs);
        adjust_outputs(&mut tb, self.address(), new_base_amount);

        let mut tx = tb.build()?;

        self.sign_transaction(&mut tx)?;

        Ok(tx)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for WalletUnlocked {
    type Error = WalletError;
    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> WalletResult<Signature> {
        let message = Message::new(message);
        let sig = Signature::sign(&self.private_key, &message);
        Ok(sig)
    }

    fn sign_transaction(&self, tx: &mut impl Transaction) -> WalletResult<Signature> {
        let consensus_parameters = self
            .try_provider()
            .map_err(|_| WalletError::NoProviderError)?
            .consensus_parameters();
        let id = tx.id(&consensus_parameters);

        let message = Message::from_bytes(*id);
        let sig = Signature::sign(&self.private_key, &message);

        let witness = vec![Witness::from(sig.as_ref())];

        let witnesses: &mut Vec<Witness> = tx.witnesses_mut();

        match witnesses.len() {
            0 => *witnesses = witness,
            _ => {
                witnesses.extend(witness);
            }
        }

        Ok(sig)
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .finish()
    }
}

impl ops::Deref for WalletUnlocked {
    type Target = Wallet;
    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

/// Generates a random mnemonic phrase given a random number generator and the number of words to
/// generate, `count`.
pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> WalletResult<String> {
    Ok(fuel_crypto::generate_mnemonic_phrase(rng, count)?)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn encrypted_json_keystore() -> Result<()> {
        let dir = tempdir()?;
        let mut rng = rand::thread_rng();

        // Create a wallet to be stored in the keystore.
        let (wallet, uuid) = WalletUnlocked::new_from_keystore(&dir, &mut rng, "password", None)?;

        // sign a message using the above key.
        let message = "Hello there!";
        let signature = wallet.sign_message(message).await?;

        // Read from the encrypted JSON keystore and decrypt it.
        let path = Path::new(dir.path()).join(uuid);
        let recovered_wallet = WalletUnlocked::load_keystore(path.clone(), "password", None)?;

        // Sign the same message as before and assert that the signature is the same.
        let signature2 = recovered_wallet.sign_message(message).await?;
        assert_eq!(signature, signature2);

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn mnemonic_generation() -> Result<()> {
        let mnemonic = generate_mnemonic_phrase(&mut rand::thread_rng(), 12)?;

        let _wallet = WalletUnlocked::new_from_mnemonic_phrase(&mnemonic, None)?;
        Ok(())
    }

    #[tokio::test]
    async fn wallet_from_mnemonic_phrase() -> Result<()> {
        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create first account from mnemonic phrase.
        let wallet =
            WalletUnlocked::new_from_mnemonic_phrase_with_path(phrase, None, "m/44'/60'/0'/0/0")?;

        let expected_plain_address =
            "df9d0e6c6c5f5da6e82e5e1a77974af6642bdb450a10c43f0c6910a212600185";
        let expected_address = "fuel1m7wsumrvtaw6d6pwtcd809627ejzhk69pggvg0cvdyg2yynqqxzseuzply";

        assert_eq!(wallet.address().hash().to_string(), expected_plain_address);
        assert_eq!(wallet.address().to_string(), expected_address);

        // Create a second account from the same phrase.
        let wallet2 =
            WalletUnlocked::new_from_mnemonic_phrase_with_path(phrase, None, "m/44'/60'/1'/0/0")?;

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
    async fn encrypt_and_store_wallet_from_mnemonic() -> Result<()> {
        let dir = tempdir()?;

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create first account from mnemonic phrase.
        let wallet =
            WalletUnlocked::new_from_mnemonic_phrase_with_path(phrase, None, "m/44'/60'/0'/0/0")?;

        let uuid = wallet.encrypt(&dir, "password")?;

        let path = Path::new(dir.path()).join(uuid);

        let recovered_wallet = WalletUnlocked::load_keystore(&path, "password", None)?;

        assert_eq!(wallet.address(), recovered_wallet.address());

        // Remove tempdir.
        assert!(std::fs::remove_file(&path).is_ok());
        Ok(())
    }
}
