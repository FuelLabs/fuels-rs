use crate::provider::Provider;
use crate::Signer;
use async_trait::async_trait;
use elliptic_curve::rand_core;
use eth_keystore::KeystoreError;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_gql_client::client::schema;
use fuel_gql_client::fuel_vm::prelude::GTFArgs;
use fuel_gql_client::{
    client::{schema::coin::Coin, types::TransactionResponse, PaginatedResult, PaginationRequest},
    fuel_tx::{
        AssetId, Bytes32, ContractId, Input, Output, Receipt, Transaction, TransactionFee,
        TxPointer, UtxoId, Witness,
    },
    fuel_vm::{consts::REG_ONE, prelude::Opcode},
};
use fuel_types::bytes::WORD_SIZE;
use fuels_core::{constants::BASE_ASSET_ID, parameters::TxParameters};
use fuels_types::bech32::{Bech32Address, Bech32ContractId, FUEL_BECH32_HRP};
use fuels_types::errors::Error;
use rand::{CryptoRng, Rng};
use std::{collections::HashMap, fmt, ops, path::Path};
use thiserror::Error;

const DEFAULT_DERIVATION_PATH_PREFIX: &str = "m/44'/1179993420'/0'/0/";

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
///
/// # Examples
///
/// ## Signing and Verifying a message
///
/// The wallet can be used to produce ECDSA [`Signature`] objects, which can be then verified.
///
/// ```
/// use fuel_crypto::Message;
/// use fuels::prelude::*;
///
/// async fn foo() -> Result<(), Error> {
///   // Setup local test node
///   let (provider, _) = setup_test_provider(vec![], vec![], None).await;
///
///   // Create a new local wallet with the newly generated key
///   let wallet = WalletUnlocked::new_random(Some(provider));
///
///   let message = "my message";
///   let signature = wallet.sign_message(message.as_bytes()).await?;
///
///   // Lock the wallet when we're done, dropping the private key from memory.
///   let wallet = wallet.lock();
///
///   // Recover address that signed the message
///   let message = Message::new(message);
///   let recovered_address = signature.recover(&message).expect("Failed to recover address");
///
///   assert_eq!(wallet.address().hash(), recovered_address.hash());
///
///   // Verify signature
///   signature.verify(&recovered_address, &message).unwrap();
///   Ok(())
/// }
/// ```
/// [`Signature`]: fuels_core::signature::Signature
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

    pub fn get_provider(&self) -> Result<&Provider, WalletError> {
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
    ) -> Result<PaginatedResult<TransactionResponse, String>, Error> {
        self.get_provider()?
            .get_transactions_by_owner(&self.address, request)
            .await
            .map_err(Into::into)
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
    ) -> Result<Vec<Input>, Error> {
        let spendable = self.get_spendable_coins(asset_id, amount).await?;
        let mut inputs = vec![];
        for coin in spendable {
            let input_coin = Input::coin_signed(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                asset_id,
                TxPointer::default(),
                witness_index,
                0,
            );
            inputs.push(input_coin);
        }
        Ok(inputs)
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

    /// Gets all coins of asset `asset_id` owned by the wallet, *even spent ones* (this is useful
    /// for some particular cases, but in general, you should use `get_spendable_coins`). This
    /// returns actual coins (UTXOs).
    pub async fn get_coins(&self, asset_id: AssetId) -> Result<Vec<Coin>, Error> {
        Ok(self
            .get_provider()?
            .get_coins(&self.address, asset_id)
            .await?)
    }

    /// Get some spendable coins of asset `asset_id` owned by the wallet that add up at least to
    /// amount `amount`. The returned coins (UTXOs) are actual coins that can be spent. The number
    /// of coins (UXTOs) is optimized to prevent dust accumulation.
    pub async fn get_spendable_coins(
        &self,
        asset_id: AssetId,
        amount: u64,
    ) -> Result<Vec<Coin>, Error> {
        self.get_provider()?
            .get_spendable_coins(&self.address, asset_id, amount)
            .await
            .map_err(Into::into)
    }

    /// Get the balance of all spendable coins `asset_id` for address `address`. This is different
    /// from getting coins because we are just returning a number (the sum of UTXOs amount) instead
    /// of the UTXOs.
    pub async fn get_asset_balance(&self, asset_id: &AssetId) -> Result<u64, Error> {
        self.get_provider()?
            .get_asset_balance(&self.address, *asset_id)
            .await
            .map_err(Into::into)
    }

    /// Get all the spendable balances of all assets for the wallet. This is different from getting
    /// the coins because we are only returning the sum of UTXOs coins amount and not the UTXOs
    /// coins themselves.
    pub async fn get_balances(&self) -> Result<HashMap<String, u64>, Error> {
        self.get_provider()?
            .get_balances(&self.address)
            .await
            .map_err(Into::into)
    }

    pub async fn get_messages(&self) -> Result<Vec<schema::message::Message>, Error> {
        Ok(self.get_provider()?.get_messages(&self.address).await?)
    }

    pub async fn get_inputs_for_messages(&self, witness_index: u8) -> Result<Vec<Input>, Error> {
        let to_u8_bytes = |v: &[i32]| v.iter().flat_map(|e| e.to_ne_bytes()).collect::<Vec<_>>();

        let messages = self.get_messages().await?;

        let inputs: Vec<Input> = messages
            .into_iter()
            .map(|message| {
                let data = to_u8_bytes(&message.data);
                let message_id = Input::compute_message_id(
                    &message.sender.clone().into(),
                    &message.recipient.clone().into(),
                    message.nonce.into(),
                    &message.owner.clone().into(),
                    message.amount.0,
                    &data,
                );
                Input::message_signed(
                    message_id,
                    message.sender.into(),
                    message.recipient.into(),
                    message.amount.0,
                    0,
                    message.owner.into(),
                    witness_index,
                    data,
                )
            })
            .collect();

        Ok(inputs)
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

    /// Craft a transaction used to transfer funds between two addresses.
    pub fn build_transfer_tx(
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Transaction {
        // This script contains a single Opcode that returns immediately (RET)
        // since all this transaction does is move Inputs and Outputs around.
        let script = Opcode::RET(REG_ONE).to_bytes().to_vec();
        Transaction::Script {
            gas_price: params.gas_price,
            gas_limit: params.gas_limit,
            maturity: params.maturity,
            receipts_root: Default::default(),
            script,
            script_data: vec![],
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
        }
    }

    /// Craft a transaction used to transfer funds to a contract.
    pub fn build_contract_transfer_tx(
        to: ContractId,
        amount: u64,
        asset_id: AssetId,
        inputs: &[Input],
        outputs: &[Output],
        params: TxParameters,
    ) -> Transaction {
        let script_data: Vec<u8> = [
            to.to_vec(),
            amount.to_be_bytes().to_vec(),
            asset_id.to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // This script loads:
        //  - a pointer to the contract id,
        //  - the actual amount
        //  - a pointer to the asset id
        // into the registers 0X10, 0x11, 0x12
        // and calls the TR instruction
        let script = vec![
            Opcode::gtf(0x10, 0x00, GTFArgs::ScriptData),
            Opcode::ADDI(0x11, 0x10, ContractId::LEN as u16),
            Opcode::LW(0x13, 0x11, 0),
            Opcode::ADDI(0x12, 0x11, WORD_SIZE as u16),
            Opcode::TR(0x10, 0x11, 0x12),
            Opcode::RET(REG_ONE),
        ]
        .into_iter()
        .collect();

        Transaction::Script {
            gas_price: params.gas_price,
            gas_limit: params.gas_limit,
            maturity: params.maturity,
            receipts_root: Default::default(),
            script,
            script_data,
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            witnesses: vec![],
            metadata: None,
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
    ) -> Result<Self, WalletError> {
        let path = format!("{}{}", DEFAULT_DERIVATION_PATH_PREFIX, 0);
        Self::new_from_mnemonic_phrase_with_path(phrase, provider, &path)
    }

    /// Creates a new wallet from a mnemonic phrase.
    /// It takes a path to a BIP32 derivation path.
    pub fn new_from_mnemonic_phrase_with_path(
        phrase: &str,
        provider: Option<Provider>,
        path: &str,
    ) -> Result<Self, WalletError> {
        let secret_key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, path)?;

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

    /// Add base asset inputs to the transaction to cover the estimated fee.
    /// The original base asset amount cannot be calculated reliably from
    /// the existing transaction inputs because the selected coins may exceed
    /// the required amount to avoid dust. Therefore we require it as an argument.
    ///
    /// Requires contract inputs to be at the start of the transactions inputs vec
    /// so that their indexes are retained
    pub async fn add_fee_coins(
        &self,
        tx: &mut Transaction,
        previous_base_amount: u64,
        witness_index: u8,
    ) -> Result<(), Error> {
        let consensus_parameters = self
            .get_provider()?
            .chain_info()
            .await?
            .consensus_parameters;
        let transaction_fee = TransactionFee::checked_from_tx(&consensus_parameters.into(), &*tx)
            .expect("Error calculating TransactionFee");

        let (base_asset_inputs, remaining_inputs): (Vec<_>, Vec<_>) =
            tx.inputs().iter().cloned().partition(|input| {
                matches!(input, Input::CoinSigned { .. })
                    && *input.asset_id().unwrap() == BASE_ASSET_ID
            });

        let base_inputs_sum: u64 = base_asset_inputs
            .iter()
            .map(|input| input.amount().unwrap())
            .sum();
        // either the inputs were setup incorrectly, or the passed base_asset_amount is wrong
        if base_inputs_sum < previous_base_amount {
            return Err(Error::WalletError(
                "The provided base asset amount is less than the present input coins".to_string(),
            ));
        }

        let mut new_base_amount = transaction_fee.total() + previous_base_amount;
        // If the tx doesn't consume any UTXOs, attempting to repeat it will lead to an
        // error due to non unique tx ids (e.g. repeated contract call with configured gas cost of 0).
        // Here we enforce a minimum amount on the base asset to avoid this
        const MIN_AMOUNT: u64 = 1;
        let is_using_coins = tx
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::CoinSigned { .. }));

        if !is_using_coins && new_base_amount == 0 {
            new_base_amount = MIN_AMOUNT;
        }

        // This is a temporary solution till we get update on coins_to_spend function
        // Get asset inputs for required amount expressed u Input::coin_signed or in
        // Input::message_signed depending on what we have in stock.
        // In the near future this will be replaced by one function.
        let base_coin_inputs = self
            .get_asset_inputs_for_amount(BASE_ASSET_ID, new_base_amount, witness_index)
            .await;
        let new_base_inputs =
            if base_coin_inputs.is_err() || base_coin_inputs.as_ref().unwrap().is_empty() {
                self.get_inputs_for_messages(witness_index).await?
            } else {
                base_coin_inputs.unwrap()
            };
        if new_base_inputs.is_empty() && new_base_amount != 0 {
            return Err(Error::ProviderError(
                "Response errors; enough coins could not be found".to_string(),
            ));
        }

        let is_using_messages = new_base_inputs
            .iter()
            .any(|input| matches!(input, Input::MessageSigned { .. }));

        let adjusted_inputs: Vec<_> = remaining_inputs
            .into_iter()
            .chain(new_base_inputs.into_iter())
            .collect();

        let is_base_change_present = tx.outputs().iter().any(|output| {
            matches!(output, Output::Change { .. }) && *output.asset_id().unwrap() == BASE_ASSET_ID
        });

        // add a change output for the base asset if it doesn't exist and there are base inputs
        let change_output = if !is_base_change_present && new_base_amount != 0 && !is_using_messages
        {
            vec![Output::change(self.address().into(), 0, BASE_ASSET_ID)]
        } else {
            vec![]
        };

        match tx {
            Transaction::Script {
                ref mut inputs,
                ref mut outputs,
                ..
            } => {
                *inputs = adjusted_inputs;
                outputs.extend(change_output);
            }
            Transaction::Create {
                ref mut inputs,
                ref mut outputs,
                ..
            } => {
                *inputs = adjusted_inputs;
                outputs.extend(change_output);
            }
        };

        Ok(())
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
    ///  let mut wallet_1 = WalletUnlocked::new_random(None);
    ///  let mut wallet_2 = WalletUnlocked::new_random(None).lock();
    ///
    ///   // Setup a coin for each wallet
    ///   let mut coins_1 = setup_single_asset_coins(wallet_1.address(),BASE_ASSET_ID, 1, 1);
    ///   let coins_2 = setup_single_asset_coins(wallet_2.address(),BASE_ASSET_ID, 1, 1);
    ///   coins_1.extend(coins_2);
    ///
    ///   // Setup a provider and node with both set of coins
    ///   let (provider, _) = setup_test_provider(coins_1, vec![], None).await;
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
    ///   let wallet_2_final_coins = wallet_2.get_coins(BASE_ASSET_ID).await.unwrap();
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
    ) -> Result<(String, Vec<Receipt>), Error> {
        let inputs = self
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?;
        let outputs = self.get_asset_outputs_for_amount(to, asset_id, amount);

        let mut tx = Wallet::build_transfer_tx(&inputs, &outputs, tx_parameters);

        // if we are not transferring the base asset, previous base amount is 0
        if asset_id == AssetId::default() {
            self.add_fee_coins(&mut tx, amount, 0).await?;
        } else {
            self.add_fee_coins(&mut tx, 0, 0).await?;
        };
        self.sign_transaction(&mut tx).await?;

        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx.id().to_string(), receipts))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spend_predicate(
        &self,
        predicate_address: &Bech32Address,
        code: Vec<u8>,
        amount: u64,
        asset_id: AssetId,
        to: &Bech32Address,
        data: Option<Vec<u8>>,
        tx_parameters: TxParameters,
    ) -> Result<Vec<Receipt>, Error> {
        let spendable_predicate_coins = self
            .get_provider()?
            .get_spendable_coins(predicate_address, asset_id, amount)
            .await?;

        // input amount is: amount < input_amount < 2*amount
        // because of "random improve" used by get_spendable_coins()
        let input_amount: u64 = spendable_predicate_coins
            .iter()
            .map(|coin| coin.amount.0)
            .sum();

        let predicate_data = data.unwrap_or_default();
        let inputs = spendable_predicate_coins
            .into_iter()
            .map(|coin| {
                Input::coin_predicate(
                    UtxoId::from(coin.utxo_id),
                    coin.owner.into(),
                    coin.amount.0,
                    asset_id,
                    TxPointer::default(),
                    0,
                    code.clone(),
                    predicate_data.clone(),
                )
            })
            .collect::<Vec<_>>();

        let outputs = [
            Output::coin(to.into(), amount, asset_id),
            Output::coin(predicate_address.into(), input_amount - amount, asset_id),
        ];

        let mut tx = Wallet::build_transfer_tx(&inputs, &outputs, tx_parameters);
        // we set previous base amount to 0 because it only applies to signed coins, not predicate coins
        self.add_fee_coins(&mut tx, 0, 0).await?;
        self.sign_transaction(&mut tx).await?;

        self.get_provider()?.send_transaction(&tx).await
    }

    pub async fn receive_from_predicate(
        &self,
        predicate_address: &Bech32Address,
        predicate_code: Vec<u8>,
        amount: u64,
        asset_id: AssetId,
        predicate_data: Option<Vec<u8>>,
        tx_parameters: TxParameters,
    ) -> Result<Vec<Receipt>, Error> {
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
    ) -> Result<(String, Vec<Receipt>), Error> {
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
        let mut tx = Wallet::build_contract_transfer_tx(
            plain_contract_id,
            balance,
            asset_id,
            &inputs,
            &outputs,
            tx_parameters,
        );
        // if we are not transferring the base asset, previous base amount is 0
        let base_amount = if asset_id == AssetId::default() {
            balance
        } else {
            0
        };
        self.add_fee_coins(&mut tx, base_amount, 0).await?;
        self.sign_transaction(&mut tx).await?;

        let receipts = self.get_provider()?.send_transaction(&tx).await?;

        Ok((tx.id().to_string(), receipts))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for WalletUnlocked {
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

impl ops::Deref for WalletUnlocked {
    type Target = Wallet;
    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

/// Generates a random mnemonic phrase given a random number generator and the number of words to
/// generate, `count`.
pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> Result<String, WalletError> {
    Ok(fuel_crypto::FuelMnemonic::generate_mnemonic_phrase(
        rng, count,
    )?)
}

#[cfg(test)]
#[cfg(feature = "test-helpers")]
mod tests {
    use super::*;
    use core::iter::repeat;
    use fuel_core::service::{Config, FuelService};
    use fuel_gql_client::client::FuelClient;
    use fuel_gql_client::fuel_tx::Address;
    use fuels_test_helpers::{launch_custom_provider_and_get_wallets, AssetConfig, WalletsConfig};
    use fuels_types::errors::Error;
    use tempfile::tempdir;

    #[tokio::test]
    async fn encrypted_json_keystore() -> Result<(), Error> {
        let dir = tempdir()?;
        let mut rng = rand::thread_rng();

        let provider = setup().await;

        // Create a wallet to be stored in the keystore.
        let (wallet, uuid) =
            WalletUnlocked::new_from_keystore(&dir, &mut rng, "password", Some(provider.clone()))?;

        // sign a message using the above key.
        let message = "Hello there!";
        let signature = wallet.sign_message(message).await?;

        // Read from the encrypted JSON keystore and decrypt it.
        let path = Path::new(dir.path()).join(uuid);
        let recovered_wallet =
            WalletUnlocked::load_keystore(&path.clone(), "password", Some(provider.clone()))?;

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

        let mnemonic = generate_mnemonic_phrase(&mut rand::thread_rng(), 12)?;

        let _wallet = WalletUnlocked::new_from_mnemonic_phrase(&mnemonic, Some(provider))?;
        Ok(())
    }

    #[tokio::test]
    async fn wallet_from_mnemonic_phrase() -> Result<(), Error> {
        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        let provider = setup().await;

        // Create first account from mnemonic phrase.
        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
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
        let wallet2 = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            phrase,
            Some(provider),
            "m/44'/60'/1'/0/0",
        )?;

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
        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            phrase,
            Some(provider.clone()),
            "m/44'/60'/0'/0/0",
        )?;

        let uuid = wallet.encrypt(&dir, "password")?;

        let path = Path::new(dir.path()).join(uuid);

        let recovered_wallet = WalletUnlocked::load_keystore(&path, "password", Some(provider))?;

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

    fn add_fee_coins_wallet_config(num_wallets: u64) -> WalletsConfig {
        let asset_configs = vec![AssetConfig {
            id: BASE_ASSET_ID,
            num_coins: 20,
            coin_amount: 20,
        }];
        WalletsConfig::new_multiple_assets(num_wallets, asset_configs)
    }

    fn compare_inputs(inputs: &[Input], expected_inputs: &mut Vec<Input>) -> bool {
        let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);

        // change UTXO_ids to 0s for comparison, because we can't guess the genesis coin ids
        let inputs: Vec<Input> = inputs
            .iter()
            .map(|input| match input {
                Input::CoinSigned {
                    owner,
                    amount,
                    asset_id,
                    tx_pointer,
                    witness_index,
                    maturity,
                    ..
                } => Input::coin_signed(
                    zero_utxo_id,
                    *owner,
                    *amount,
                    *asset_id,
                    *tx_pointer,
                    *witness_index,
                    *maturity,
                ),
                other => other.clone(),
            })
            .collect();

        let comparison_results: Vec<bool> = inputs
            .iter()
            .map(|input| {
                let found_index = expected_inputs
                    .iter()
                    .position(|expected| expected == input);
                if let Some(index) = found_index {
                    expected_inputs.remove(index);
                    true
                } else {
                    false
                }
            })
            .collect();

        if !expected_inputs.is_empty() {
            return false;
        }

        return comparison_results.iter().all(|&r| r);
    }

    #[tokio::test]
    async fn add_fee_coins_empty_transaction() -> Result<(), Error> {
        let wallet_config = add_fee_coins_wallet_config(1);
        let wallet = launch_custom_provider_and_get_wallets(wallet_config, None)
            .await
            .pop()
            .unwrap();
        let mut tx = Wallet::build_transfer_tx(&[], &[], TxParameters::default());

        wallet.add_fee_coins(&mut tx, 0, 0).await?;

        let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);
        let mut expected_inputs = vec![Input::coin_signed(
            zero_utxo_id,
            wallet.address().into(),
            20,
            BASE_ASSET_ID,
            TxPointer::default(),
            0,
            0,
        )];
        let expected_outputs = vec![Output::change(wallet.address().into(), 0, BASE_ASSET_ID)];

        assert!(compare_inputs(tx.inputs(), &mut expected_inputs));
        assert_eq!(tx.outputs(), expected_outputs);

        Ok(())
    }

    #[tokio::test]
    async fn add_fee_coins_to_transfer_with_base_asset() -> Result<(), Error> {
        let wallet_config = add_fee_coins_wallet_config(1);
        let wallet = launch_custom_provider_and_get_wallets(wallet_config, None)
            .await
            .pop()
            .unwrap();

        let base_amount = 30;
        let inputs = wallet
            .get_asset_inputs_for_amount(BASE_ASSET_ID, base_amount, 0)
            .await?;
        let outputs = wallet.get_asset_outputs_for_amount(
            &Address::zeroed().into(),
            BASE_ASSET_ID,
            base_amount,
        );
        let mut tx = Wallet::build_transfer_tx(&inputs, &outputs, TxParameters::default());

        wallet.add_fee_coins(&mut tx, base_amount, 0).await?;

        let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);
        let mut expected_inputs = repeat(Input::coin_signed(
            zero_utxo_id,
            wallet.address().into(),
            20,
            BASE_ASSET_ID,
            TxPointer::default(),
            0,
            0,
        ))
        .take(3)
        .collect::<Vec<_>>();
        let expected_outputs = vec![
            Output::coin(Address::zeroed(), base_amount, BASE_ASSET_ID),
            Output::change(wallet.address().into(), 0, BASE_ASSET_ID),
        ];

        assert!(compare_inputs(tx.inputs(), &mut expected_inputs));
        assert_eq!(tx.outputs(), expected_outputs);

        Ok(())
    }
}
