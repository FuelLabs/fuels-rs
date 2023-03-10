use std::{collections::HashMap, fmt, ops, path::Path};

use async_trait::async_trait;
use elliptic_curve::rand_core;
use eth_keystore::KeystoreError;
use fuel_core_client::client::{PaginatedResult, PaginationRequest};
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_tx::{AssetId, Bytes32, ContractId, Input, Output, Receipt, TxPointer, UtxoId, Witness};
use fuel_types::MessageId;
use fuels_core::{
    abi_encoder::UnresolvedBytes,
    offsets::{base_offset, coin_predicate_data_offset, message_predicate_data_offset},
};
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId, FUEL_BECH32_HRP},
    coin::Coin,
    constants::BASE_ASSET_ID,
    errors::{error, Error, Result},
    message::Message as InputMessage,
    parameters::TxParameters,
    resource::Resource,
    transaction::{ScriptTransaction, Transaction},
    transaction_response::TransactionResponse,
};
use rand::{CryptoRng, Rng};
use thiserror::Error;

use crate::{
    provider::{Provider, ResourceFilter},
    Signer,
};

pub const DEFAULT_DERIVATION_PATH_PREFIX: &str = "m/44'/1179993420'";

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
    pub(crate) provider: Option<Provider>,
}

/// A `WalletUnlocked` is equivalent to a [`Wallet`] whose private key is known and stored
/// alongside in-memory. Knowing the private key allows a `WalletUlocked` to sign operations, send
/// transactions, and more.
#[derive(Clone, Debug)]
pub struct WalletUnlocked {
    wallet: Wallet,
    pub(crate) private_key: SecretKey,
}

#[derive(Error, Debug)]
/// Error thrown by the Wallet module
pub enum WalletError {
    /// Error propagated from the hex crate.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Error propagated by parsing of a slice
    #[error("Failed to parse slice")]
    Parsing(#[from] std::array::TryFromSliceError),
    #[error("No provider was setup: make sure to set_provider in your wallet!")]
    NoProvider,
    /// Keystore error
    #[error(transparent)]
    KeystoreError(#[from] KeystoreError),
    #[error(transparent)]
    FuelCrypto(#[from] fuel_crypto::Error),
}

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::WalletError(e.to_string())
    }
}

impl Wallet {
    /// Construct a Wallet from its given public address.
    pub fn from_address(address: Bech32Address, provider: Option<Provider>) -> Self {
        Self { address, provider }
    }

    pub fn get_provider(&self) -> WalletResult<&Provider> {
        self.provider.as_ref().ok_or(WalletError::NoProvider)
    }

    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = Some(provider)
    }

    pub fn address(&self) -> &Bech32Address {
        &self.address
    }

    pub async fn get_transactions(
        &self,
        request: PaginationRequest<String>,
    ) -> Result<PaginatedResult<TransactionResponse, String>> {
        Ok(self
            .get_provider()?
            .get_transactions_by_owner(&self.address, request)
            .await?)
    }

    /// Returns a vector consisting of `Input::Coin`s and `Input::Message`s for the given
    /// asset ID and amount. The `witness_index` is the position of the witness (signature)
    /// in the transaction's list of witnesses. In the validation process, the node will
    /// use the witness at this index to validate the coins returned by this method.
    pub async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        witness_index: u8,
    ) -> Result<Vec<Input>> {
        let filter = ResourceFilter {
            from: self.address().clone(),
            asset_id,
            amount,
            ..Default::default()
        };
        self.get_provider()?
            .get_asset_inputs(filter, witness_index)
            .await
    }

    /// Returns a vector containing the output coin and change output given an asset and amount
    pub fn get_asset_outputs_for_amount(
        &self,
        to: &Bech32Address,
        asset_id: AssetId,
        amount: u64,
    ) -> Vec<Output> {
        vec![
            Output::coin(to.into(), amount, asset_id),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            Output::change((&self.address).into(), 0, asset_id),
        ]
    }

    /// Gets all unspent coins of asset `asset_id` owned by the wallet.
    pub async fn get_coins(&self, asset_id: AssetId) -> Result<Vec<Coin>> {
        Ok(self
            .get_provider()?
            .get_coins(&self.address, asset_id)
            .await?)
    }

    /// Get some spendable resources (coins and messages) of asset `asset_id` owned by the wallet
    /// that add up at least to amount `amount`. The returned coins (UTXOs) are actual coins that
    /// can be spent. The number of UXTOs is optimized to prevent dust accumulation.
    pub async fn get_spendable_resources(
        &self,
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Resource>> {
        let filter = ResourceFilter {
            from: self.address().clone(),
            asset_id,
            amount,
            ..Default::default()
        };
        self.get_provider()?
            .get_spendable_resources(filter)
            .await
            .map_err(Into::into)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(&self, asset_id: &AssetId) -> Result<u64> {
        self.get_provider()?
            .get_asset_balance(&self.address, *asset_id)
            .await
            .map_err(Into::into)
    }

    /// Get all the spendable balances of all assets for the wallet. This is different from getting
    /// the coins because we are only returning the sum of UTXOs coins amount and not the UTXOs
    /// coins themselves.
    pub async fn get_balances(&self) -> Result<HashMap<String, u64>> {
        self.get_provider()?
            .get_balances(&self.address)
            .await
            .map_err(Into::into)
    }

    pub async fn get_messages(&self) -> Result<Vec<InputMessage>> {
        Ok(self.get_provider()?.get_messages(&self.address).await?)
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

impl WalletUnlocked {
    /// Lock the wallet by `drop`ping the private key from memory.
    pub fn lock(self) -> Wallet {
        self.wallet
    }

    // NOTE: Rather than providing a `DerefMut` implementation, we wrap the `set_provider` method
    // directly. This is because we should not allow the user a `&mut` handle to the inner `Wallet`
    // as this could lead to ending up with a `WalletUnlocked` in an inconsistent state (e.g. the
    // private key doesn't match the inner wallet's public key).
    pub fn set_provider(&mut self, provider: Provider) {
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
        let (secret, uuid) = eth_keystore::new(dir, rng, password)?;

        let secret_key = unsafe { SecretKey::from_slice_unchecked(&secret) };

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
        let secret_key = unsafe { SecretKey::from_slice_unchecked(&secret) };
        Ok(Self::new_from_private_key(secret_key, provider))
    }

    /// Add base asset inputs to the transaction to cover the estimated fee.
    /// The original base asset amount cannot be calculated reliably from
    /// the existing transaction inputs because the selected resources may exceed
    /// the required amount to avoid dust. Therefore we require it as an argument.
    ///
    /// Requires contract inputs to be at the start of the transactions inputs vec
    /// so that their indexes are retained
    pub async fn add_fee_resources(
        &self,
        tx: &mut impl Transaction,
        previous_base_amount: u64,
        witness_index: u8,
    ) -> Result<()> {
        let consensus_parameters = self
            .get_provider()?
            .chain_info()
            .await?
            .consensus_parameters;
        let transaction_fee = tx
            .fee_checked_from_tx(&consensus_parameters)
            .expect("Error calculating TransactionFee");

        let (base_asset_inputs, remaining_inputs): (Vec<_>, Vec<_>) =
            tx.inputs().iter().cloned().partition(|input| {
                matches!(input, Input::MessageSigned { .. })
                || matches!(input, Input::CoinSigned { asset_id, .. } if asset_id == &BASE_ASSET_ID)
            });

        let base_inputs_sum: u64 = base_asset_inputs
            .iter()
            .map(|input| input.amount().unwrap())
            .sum();
        // either the inputs were setup incorrectly, or the passed previous_base_amount is wrong
        if base_inputs_sum < previous_base_amount {
            return Err(error!(
                WalletError,
                "The provided base asset amount is less than the present input coins"
            ));
        }

        let mut new_base_amount = transaction_fee.total() + previous_base_amount;
        // If the tx doesn't consume any UTXOs, attempting to repeat it will lead to an
        // error due to non unique tx ids (e.g. repeated contract call with configured gas cost of 0).
        // Here we enforce a minimum amount on the base asset to avoid this
        let is_consuming_utxos = tx
            .inputs()
            .iter()
            .any(|input| !matches!(input, Input::Contract { .. }));
        const MIN_AMOUNT: u64 = 1;
        if !is_consuming_utxos && new_base_amount == 0 {
            new_base_amount = MIN_AMOUNT;
        }

        let new_base_inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, new_base_amount, witness_index)
            .await?;
        let adjusted_inputs: Vec<_> = remaining_inputs
            .into_iter()
            .chain(new_base_inputs.into_iter())
            .collect();
        *tx.inputs_mut() = adjusted_inputs;

        let is_base_change_present = tx.outputs().iter().any(|output| {
            matches!(output, Output::Change { asset_id, .. } if asset_id == &BASE_ASSET_ID)
        });
        // add a change output for the base asset if it doesn't exist and there are base inputs
        if !is_base_change_present && new_base_amount != 0 {
            tx.outputs_mut()
                .push(Output::change(self.address().into(), 0, BASE_ASSET_ID));
        }

        Ok(())
    }

    /// Transfer funds from this wallet to another `Address`.
    /// Fails if amount for asset ID is larger than address's spendable coins.
    /// Returns the transaction ID that was sent and the list of receipts.
    pub async fn transfer(
        &self,
        to: &Bech32Address,
        amount: u64,
        asset_id: AssetId,
        tx_parameters: TxParameters,
    ) -> Result<(String, Vec<Receipt>)> {
        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;
        let outputs = self.get_asset_outputs_for_amount(to, asset_id, amount);

        let mut tx = ScriptTransaction::new(inputs, outputs, tx_parameters);

        // if we are not transferring the base asset, previous base amount is 0
        if asset_id == AssetId::default() {
            self.add_fee_resources(&mut tx, amount, 0).await?;
        } else {
            self.add_fee_resources(&mut tx, 0, 0).await?;
        };
        self.sign_transaction(&mut tx).await?;

        let tx_id = tx.id().to_string();
        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx_id, receipts))
    }

    /// Withdraws an amount of the base asset to
    /// an address on the base chain.
    /// Returns the transaction ID, message ID and the list of receipts.
    pub async fn withdraw_to_base_layer(
        &self,
        to: &Bech32Address,
        amount: u64,
        tx_parameters: TxParameters,
    ) -> Result<(String, String, Vec<Receipt>)> {
        let inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, amount, 0)
            .await?;

        let mut tx =
            ScriptTransaction::build_message_to_output_tx(to.into(), amount, inputs, tx_parameters);

        self.add_fee_resources(&mut tx, amount, 0).await?;
        self.sign_transaction(&mut tx).await?;

        let tx_id = tx.id().to_string();
        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        let message_id = WalletUnlocked::extract_message_id(&receipts)
            .expect("MessageId could not be retrieved from tx receipts.");

        Ok((tx_id, message_id.to_string(), receipts))
    }

    fn extract_message_id(receipts: &[Receipt]) -> Option<&MessageId> {
        receipts
            .iter()
            .find(|r| matches!(r, Receipt::MessageOut { .. }))
            .and_then(|m| m.message_id())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spend_predicate(
        &self,
        predicate_address: &Bech32Address,
        code: Vec<u8>,
        amount: u64,
        asset_id: AssetId,
        to: &Bech32Address,
        predicate_data: UnresolvedBytes,
        tx_parameters: TxParameters,
    ) -> Result<Vec<Receipt>> {
        let provider = self.get_provider()?;

        let filter = ResourceFilter {
            from: predicate_address.clone(),
            amount,
            ..Default::default()
        };
        let spendable_predicate_resources = provider.get_spendable_resources(filter).await?;

        // input amount is: amount < input_amount < 2*amount
        // because of "random improve" used by get_spendable_coins()
        let input_amount: u64 = spendable_predicate_resources
            .iter()
            .map(|resource| resource.amount())
            .sum();

        // Iterate through the spendable resources and calculate the appropriate offsets
        // for the coin or message predicates
        let mut offset = base_offset(&provider.consensus_parameters().await?);
        let inputs = spendable_predicate_resources
            .into_iter()
            .map(|resource| match resource {
                Resource::Coin(coin) => {
                    offset += coin_predicate_data_offset(code.len());

                    let data = predicate_data.clone().resolve(offset as u64);
                    offset += data.len();

                    self.create_coin_predicate(coin, asset_id, code.clone(), data)
                }
                Resource::Message(message) => {
                    offset += message_predicate_data_offset(message.data.len(), code.len());

                    let data = predicate_data.clone().resolve(offset as u64);
                    offset += data.len();

                    self.create_message_predicate(message, code.clone(), data)
                }
            })
            .collect::<Vec<_>>();

        let outputs = vec![
            Output::coin(to.into(), amount, asset_id),
            Output::coin(predicate_address.into(), input_amount - amount, asset_id),
        ];

        let mut tx = ScriptTransaction::new(inputs, outputs, tx_parameters);
        // we set previous base amount to 0 because it only applies to signed coins, not predicate coins
        self.add_fee_resources(&mut tx, 0, 0).await?;
        self.sign_transaction(&mut tx).await?;

        provider.send_transaction(&tx).await
    }

    fn create_coin_predicate(
        &self,
        coin: Coin,
        asset_id: AssetId,
        code: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Input {
        Input::coin_predicate(
            coin.utxo_id,
            coin.owner.into(),
            coin.amount,
            asset_id,
            TxPointer::default(),
            0,
            code,
            predicate_data,
        )
    }

    fn create_message_predicate(
        &self,
        message: InputMessage,
        code: Vec<u8>,
        predicate_data: Vec<u8>,
    ) -> Input {
        Input::message_predicate(
            message.message_id(),
            message.sender.into(),
            message.recipient.into(),
            message.amount,
            message.nonce,
            message.data,
            code,
            predicate_data,
        )
    }

    pub async fn receive_from_predicate(
        &self,
        predicate_address: &Bech32Address,
        predicate_code: Vec<u8>,
        amount: u64,
        asset_id: AssetId,
        predicate_data: UnresolvedBytes,
        tx_parameters: TxParameters,
    ) -> Result<Vec<Receipt>> {
        self.spend_predicate(
            predicate_address,
            predicate_code,
            amount,
            asset_id,
            self.address(),
            predicate_data,
            tx_parameters,
        )
        .await
    }

    /// Unconditionally transfers `balance` of type `asset_id` to
    /// the contract at `to`.
    /// Fails if balance for `asset_id` is larger than this wallet's spendable balance.
    /// Returns the corresponding transaction ID and the list of receipts.
    ///
    /// CAUTION !!!
    ///
    /// This will transfer coins to a contract, possibly leading
    /// to the PERMANENT LOSS OF COINS if not used with care.
    pub async fn force_transfer_to_contract(
        &self,
        to: &Bech32ContractId,
        balance: u64,
        asset_id: AssetId,
        tx_parameters: TxParameters,
    ) -> Result<(String, Vec<Receipt>)> {
        let zeroes = Bytes32::zeroed();
        let plain_contract_id: ContractId = to.into();

        let mut inputs = vec![Input::contract(
            UtxoId::new(zeroes, 0),
            zeroes,
            zeroes,
            TxPointer::default(),
            plain_contract_id,
        )];
        inputs.extend(
            self.get_asset_inputs_for_amount(asset_id, balance, 0)
                .await?,
        );

        let outputs = vec![
            Output::contract(0, zeroes, zeroes),
            Output::change((&self.address).into(), 0, asset_id),
        ];

        // Build transaction and sign it
        let mut tx = ScriptTransaction::build_contract_transfer_tx(
            plain_contract_id,
            balance,
            asset_id,
            inputs,
            outputs,
            tx_parameters,
        );
        // if we are not transferring the base asset, previous base amount is 0
        let base_amount = if asset_id == AssetId::default() {
            balance
        } else {
            0
        };
        self.add_fee_resources(&mut tx, base_amount, 0).await?;
        self.sign_transaction(&mut tx).await?;

        let tx_id = tx.id();
        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx_id.to_string(), receipts))
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

    async fn sign_transaction<T: Transaction + Send>(&self, tx: &mut T) -> WalletResult<Signature> {
        let id = tx.id();

        // Safety: `Message::from_bytes_unchecked` is unsafe because
        // it can't guarantee that the provided bytes will be the product
        // of a cryptographically secure hash. However, the bytes are
        // coming from `tx.id()`, which already uses `Hasher::hash()`
        // to hash it using a secure hash mechanism.
        let message = unsafe { Message::from_bytes_unchecked(*id) };
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
pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> WalletResult<String> {
    Ok(fuel_crypto::FuelMnemonic::generate_mnemonic_phrase(
        rng, count,
    )?)
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
