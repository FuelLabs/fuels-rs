use std::{fmt, ops, path::Path};

use async_trait::async_trait;
use elliptic_curve::rand_core;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    traits::Signer,
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        coin_type_id::CoinTypeId,
        errors::{error, Result},
        input::Input,
        transaction_builders::TransactionBuilder,
        AssetId,
    },
};
use rand::{CryptoRng, Rng};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{accounts_utils::try_provider_error, provider::Provider, Account, ViewOnlyAccount};

pub const DEFAULT_DERIVATION_PATH_PREFIX: &str = "m/44'/1179993420'";

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
///
/// `private_key` will be zeroed out on calling `lock()` or `drop`ping a `WalletUnlocked`.
#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct WalletUnlocked {
    #[zeroize(skip)]
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

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider);
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

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ViewOnlyAccount for Wallet {
    fn address(&self) -> &Bech32Address {
        self.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.provider.as_ref().ok_or_else(try_provider_error)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        Ok(self
            .get_spendable_resources(asset_id, amount, excluded_coins)
            .await?
            .into_iter()
            .map(Input::resource_signed)
            .collect::<Vec<Input>>())
    }
}

impl WalletUnlocked {
    /// Lock the wallet by securely `zeroize`-ing and `drop`ping the private key from memory.
    pub fn lock(mut self) -> Wallet {
        self.private_key.zeroize();
        self.wallet.clone()
    }

    // NOTE: Rather than providing a `DerefMut` implementation, we wrap the `set_provider` method
    // directly. This is because we should not allow the user a `&mut` handle to the inner `Wallet`
    // as this could lead to ending up with a `WalletUnlocked` in an inconsistent state (e.g. the
    // private key doesn't match the inner wallet's public key).
    pub fn set_provider(&mut self, provider: Provider) {
        self.wallet.set_provider(provider);
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
    pub fn new_from_mnemonic_phrase(phrase: &str, provider: Option<Provider>) -> Result<Self> {
        let path = format!("{DEFAULT_DERIVATION_PATH_PREFIX}/0'/0/0");
        Self::new_from_mnemonic_phrase_with_path(phrase, provider, &path)
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// It takes a derivation path such as BIP32 or BIP44.
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        provider: Option<Provider>,
        path: &str,
    ) -> Result<Self> {
        let secret_key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, path)?;

        Ok(Self::new_from_private_key(secret_key, provider))
    }

    /// Creates a new wallet and stores its encrypted version in the given path.
    pub fn new_from_keystore<P, R, S>(
        dir: P,
        rng: &mut R,
        password: S,
        provider: Option<Provider>,
    ) -> Result<(Self, String)>
    where
        P: AsRef<Path>,
        R: Rng + CryptoRng + rand_core::CryptoRng,
        S: AsRef<[u8]>,
    {
        let (secret, uuid) =
            eth_keystore::new(dir, rng, password, None).map_err(|e| error!(Other, "{e}"))?;

        let secret_key = SecretKey::try_from(secret.as_slice()).expect("should have correct size");

        let wallet = Self::new_from_private_key(secret_key, provider);

        Ok((wallet, uuid))
    }

    /// Encrypts the wallet's private key with the given password and saves it
    /// to the given path.
    pub fn encrypt<P, S>(&self, dir: P, password: S) -> Result<String>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let mut rng = rand::thread_rng();

        eth_keystore::encrypt_key(dir, &mut rng, *self.private_key, password, None)
            .map_err(|e| error!(Other, "{e}"))
    }

    /// Recreates a wallet from an encrypted JSON wallet given the provided path and password.
    pub fn load_keystore<P, S>(keypath: P, password: S, provider: Option<Provider>) -> Result<Self>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let secret =
            eth_keystore::decrypt_key(keypath, password).map_err(|e| error!(Other, "{e}"))?;
        let secret_key = SecretKey::try_from(secret.as_slice())
            .expect("Decrypted key should have a correct size");
        Ok(Self::new_from_private_key(secret_key, provider))
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    /// Returns the private key of the wallet. This method is only available when the 'test-helpers' feature is enabled.
    #[cfg(feature = "test-helpers")]
    pub fn secret_key(&self) -> &SecretKey {
        &self.private_key
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ViewOnlyAccount for WalletUnlocked {
    fn address(&self) -> &Bech32Address {
        self.wallet.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.provider.as_ref().ok_or_else(try_provider_error)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        self.wallet
            .get_asset_inputs_for_amount(asset_id, amount, excluded_coins)
            .await
    }
}

impl Account for WalletUnlocked {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for WalletUnlocked {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let sig = Signature::sign(&self.private_key, &message);

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

impl ops::Deref for WalletUnlocked {
    type Target = Wallet;
    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

/// Generates a random mnemonic phrase given a random number generator and the number of words to
/// generate, `count`.
pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> Result<String> {
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
        let message = Message::new("Hello there!".as_bytes());
        let signature = wallet.sign(message).await?;

        // Read from the encrypted JSON keystore and decrypt it.
        let path = Path::new(dir.path()).join(uuid);
        let recovered_wallet = WalletUnlocked::load_keystore(path.clone(), "password", None)?;

        // Sign the same message as before and assert that the signature is the same.
        let signature2 = recovered_wallet.sign(message).await?;
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
