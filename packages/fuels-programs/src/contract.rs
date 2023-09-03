use std::{
    collections::HashMap,
    fmt::Debug,
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use fuel_tx::{
    AssetId, Bytes32, Contract as FuelContract, ContractId, Output, Receipt, Salt, StorageSlot,
};
use fuels_accounts::{provider::TransactionCost, Account};
use fuels_core::{
    codec::ABIEncoder,
    constants::{BASE_ASSET_ID, DEFAULT_CALL_PARAMS_AMOUNT},
    traits::{Parameterize, Tokenizable},
    types::{
        bech32::{Bech32Address, Bech32ContractId},
        errors::{error, Error, Result},
        param_types::ParamType,
        transaction::{ScriptTransaction, Transaction, TxParameters},
        transaction_builders::CreateTransactionBuilder,
        unresolved_bytes::UnresolvedBytes,
        Selector, Token,
    },
    Configurables,
};

use crate::{
    call_response::FuelCallResponse,
    call_utils::{build_tx_from_contract_calls, new_variable_outputs, TxDependencyExtension},
    logs::{map_revert_error, LogDecoder},
    receipt_parser::ReceiptParser,
};

#[derive(Debug, Clone)]
pub struct CallParameters {
    amount: u64,
    asset_id: AssetId,
    gas_forwarded: Option<u64>,
}

impl CallParameters {
    pub fn new(amount: u64, asset_id: AssetId, gas_forwarded: u64) -> Self {
        Self {
            amount,
            asset_id,
            gas_forwarded: Some(gas_forwarded),
        }
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn with_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = asset_id;
        self
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    pub fn with_gas_forwarded(mut self, gas_forwarded: u64) -> Self {
        self.gas_forwarded = Some(gas_forwarded);
        self
    }

    pub fn gas_forwarded(&self) -> Option<u64> {
        self.gas_forwarded
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: DEFAULT_CALL_PARAMS_AMOUNT,
            asset_id: BASE_ASSET_ID,
            gas_forwarded: None,
        }
    }
}

// Trait implemented by contract instances so that
// they can be passed to the `with_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

/// Configuration for contract storage
#[derive(Debug, Clone)]
pub struct StorageConfiguration {
    autoload_storage: bool,
    slot_overrides: StorageSlots,
}

impl Default for StorageConfiguration {
    fn default() -> Self {
        Self {
            autoload_storage: true,
            slot_overrides: Default::default(),
        }
    }
}

impl StorageConfiguration {
    pub fn new(autoload_enabled: bool, slots: impl IntoIterator<Item = StorageSlot>) -> Self {
        let config = Self {
            autoload_storage: autoload_enabled,
            slot_overrides: Default::default(),
        };

        config.add_slot_overrides(slots)
    }

    /// If enabled will try to automatically discover and load the storage configuration from the
    /// storage config json file.
    pub fn with_autoload(mut self, enabled: bool) -> Self {
        self.autoload_storage = enabled;
        self
    }

    pub fn autoload_enabled(&self) -> bool {
        self.autoload_storage
    }

    /// Slots added via [`add_slot_overrides`] will override any
    /// existing slots with matching keys.
    pub fn add_slot_overrides(
        mut self,
        storage_slots: impl IntoIterator<Item = StorageSlot>,
    ) -> Self {
        self.slot_overrides.add_overrides(storage_slots);
        self
    }

    /// Slots added via [`add_slot_overrides_from_file`] will override any
    /// existing slots with matching keys.
    ///
    /// `path` - path to a JSON file containing the storage slots.
    pub fn add_slot_overrides_from_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let slots = StorageSlots::load_from_file(path.as_ref())?;
        self.slot_overrides.add_overrides(slots.into_iter());
        Ok(self)
    }

    pub fn into_slots(self) -> impl Iterator<Item = StorageSlot> {
        self.slot_overrides.into_iter()
    }
}

#[derive(Debug, Clone, Default)]
struct StorageSlots {
    storage_slots: HashMap<Bytes32, StorageSlot>,
}

impl StorageSlots {
    fn from(storage_slots: impl IntoIterator<Item = StorageSlot>) -> Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        Self {
            storage_slots: pairs.collect(),
        }
    }

    fn add_overrides(&mut self, storage_slots: impl IntoIterator<Item = StorageSlot>) -> &mut Self {
        let pairs = storage_slots.into_iter().map(|slot| (*slot.key(), slot));
        self.storage_slots.extend(pairs);
        self
    }

    fn load_from_file(storage_path: impl AsRef<Path>) -> Result<Self> {
        let storage_path = storage_path.as_ref();
        validate_path_and_extension(storage_path, "json")?;

        let storage_json_string = std::fs::read_to_string(storage_path).map_err(|e| {
            error!(
                InvalidData,
                "failed to read storage slots from: {storage_path:?}. Reason: {e}"
            )
        })?;

        let decoded_slots = serde_json::from_str::<Vec<StorageSlot>>(&storage_json_string)?;

        Ok(StorageSlots::from(decoded_slots))
    }

    fn into_iter(self) -> impl Iterator<Item = StorageSlot> {
        self.storage_slots.into_values()
    }
}

/// Configuration for contract deployment
#[derive(Debug, Clone, Default)]
pub struct LoadConfiguration {
    storage: StorageConfiguration,
    configurables: Configurables,
    salt: Salt,
}

impl LoadConfiguration {
    pub fn new(
        storage: StorageConfiguration,
        configurables: impl Into<Configurables>,
        salt: impl Into<Salt>,
    ) -> Self {
        Self {
            storage,
            configurables: configurables.into(),
            salt: salt.into(),
        }
    }

    pub fn with_storage_configuration(mut self, storage: StorageConfiguration) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        self.configurables = configurables.into();
        self
    }

    pub fn with_salt(mut self, salt: impl Into<Salt>) -> Self {
        self.salt = salt.into();
        self
    }
}

/// [`Contract`] is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
#[derive(Debug)]
pub struct Contract {
    binary: Vec<u8>,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
    contract_id: ContractId,
    code_root: Bytes32,
    state_root: Bytes32,
}

impl Contract {
    pub fn new(binary: Vec<u8>, salt: Salt, storage_slots: Vec<StorageSlot>) -> Self {
        let (contract_id, code_root, state_root) =
            Self::compute_contract_id_and_state_root(&binary, &salt, &storage_slots);

        Self {
            binary,
            salt,
            storage_slots,
            contract_id,
            code_root,
            state_root,
        }
    }

    fn compute_contract_id_and_state_root(
        binary: &[u8],
        salt: &Salt,
        storage_slots: &[StorageSlot],
    ) -> (ContractId, Bytes32, Bytes32) {
        let fuel_contract = FuelContract::from(binary);
        let code_root = fuel_contract.root();
        let state_root = FuelContract::initial_state_root(storage_slots.iter());

        let contract_id = fuel_contract.id(salt, &code_root, &state_root);

        (contract_id, code_root, state_root)
    }

    pub fn with_salt(self, salt: impl Into<Salt>) -> Self {
        Self::new(self.binary, salt.into(), self.storage_slots)
    }

    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }

    pub fn state_root(&self) -> Bytes32 {
        self.state_root
    }

    pub fn code_root(&self) -> Bytes32 {
        self.code_root
    }

    /// Deploys a compiled contract to a running node
    /// To deploy a contract, you need an account with enough assets to pay for deployment.
    /// This account will also receive the change.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_parameters: TxParameters,
    ) -> Result<Bech32ContractId> {
        let tb = CreateTransactionBuilder::prepare_contract_deployment(
            self.binary,
            self.contract_id,
            self.state_root,
            self.salt,
            self.storage_slots,
            tx_parameters,
        );

        let tx = account
            .add_fee_resources(tb, 0)
            .await
            .map_err(|err| error!(ProviderError, "{err}"))?;

        let provider = account
            .try_provider()
            .map_err(|_| error!(ProviderError, "Failed to get_provider"))?;
        provider.send_transaction(tx).await?;

        Ok(self.contract_id.into())
    }

    pub fn load_from(binary_filepath: impl AsRef<Path>, config: LoadConfiguration) -> Result<Self> {
        let binary_filepath = binary_filepath.as_ref();
        validate_path_and_extension(binary_filepath, "bin")?;

        let mut binary = fs::read(binary_filepath)
            .map_err(|_| error!(InvalidData, "failed to read binary: {binary_filepath:?}"))?;

        config.configurables.update_constants_in(&mut binary);

        let storage_slots = Self::determine_storage_slots(config.storage, binary_filepath)?;

        Ok(Self::new(binary, config.salt, storage_slots))
    }

    fn determine_storage_slots(
        storage_config: StorageConfiguration,
        binary_filepath: &Path,
    ) -> Result<Vec<StorageSlot>> {
        let autoload_enabled = storage_config.autoload_enabled();
        let user_overrides = storage_config.into_slots().collect::<Vec<_>>();
        let slots = if autoload_enabled {
            let mut slots = autoload_storage_slots(binary_filepath)?;
            slots.add_overrides(user_overrides);
            slots.into_iter().collect()
        } else {
            user_overrides
        };

        Ok(slots)
    }
}

fn autoload_storage_slots(contract_binary: &Path) -> Result<StorageSlots> {
    let storage_file = expected_storage_slots_filepath(contract_binary)
        .ok_or_else(|| error!(InvalidData, "Could not determine storage slots file"))?;

    StorageSlots::load_from_file(&storage_file)
                .map_err(|_| error!(InvalidData, "Could not autoload storage slots from file: {storage_file:?}. Either provide the file or disable autoloading in StorageConfiguration"))
}

fn expected_storage_slots_filepath(contract_binary: &Path) -> Option<PathBuf> {
    let dir = contract_binary.parent()?;

    let binary_filename = contract_binary.file_stem()?.to_str()?;

    Some(dir.join(format!("{binary_filename}-storage_slots.json")))
}

fn validate_path_and_extension(file_path: &Path, extension: &str) -> Result<()> {
    if !file_path.exists() {
        return Err(error!(InvalidData, "file {file_path:?} does not exist"));
    }

    let path_extension = file_path.extension().ok_or_else(|| {
        error!(
            InvalidData,
            "could not extract extension from: {file_path:?}"
        )
    })?;

    if extension != path_extension {
        return Err(error!(
            InvalidData,
            "expected {file_path:?} to have '.{extension}' extension"
        ));
    }

    Ok(())
}

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: UnresolvedBytes,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub compute_custom_input_offset: bool,
    pub variable_outputs: Vec<Output>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: ParamType,
    pub is_payable: bool,
    pub custom_assets: HashMap<(AssetId, Option<Bech32Address>), u64>,
}

impl ContractCall {
    pub fn with_contract_id(self, contract_id: Bech32ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn with_external_contracts(
        self,
        external_contracts: Vec<Bech32ContractId>,
    ) -> ContractCall {
        ContractCall {
            external_contracts,
            ..self
        }
    }

    pub fn with_variable_outputs(self, variable_outputs: Vec<Output>) -> ContractCall {
        ContractCall {
            variable_outputs,
            ..self
        }
    }

    pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
        ContractCall {
            call_parameters,
            ..self
        }
    }

    pub fn append_variable_outputs(&mut self, num: u64) {
        self.variable_outputs
            .extend(new_variable_outputs(num as usize));
    }

    pub fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct ContractCallHandler<T: Account, D> {
    pub contract_call: ContractCall,
    pub tx_parameters: TxParameters,
    // Initially `None`, gets set to the right tx id after the transaction is submitted
    cached_tx_id: Option<Bytes32>,
    pub account: T,
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<T, D> ContractCallHandler<T, D>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).with_contract_ids(&[another_contract_id]).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn with_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.contract_call.external_contracts = contract_ids.to_vec();
        self
    }

    /// Sets external contract instances as dependencies to this contract's call.
    /// Effectively, this will be used to: merge `LogDecoder`s and create
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`] pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).with_contracts(&[another_contract_instance]).call()
    /// ```
    pub fn with_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
        self.contract_call.external_contracts = contracts.iter().map(|c| c.id()).collect();
        for c in contracts {
            self.log_decoder.merge(c.log_decoder());
        }
        self
    }

    /// Adds a custom `asset_id` with its `amount` and an optional `address` to be used for
    /// generating outputs to this contract's call.
    ///
    /// # Parameters
    /// - `asset_id`: The unique identifier of the asset being added.
    /// - `amount`: The amount of the asset being added.
    /// - `address`: The optional account address that the output amount will be sent to.
    ///              If not provided, the asset will be sent to the users account address.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let asset_id = AssetId::from([3u8; 32]);
    /// let amount = 5000;
    /// my_contract_instance.my_method(...).add_custom_asset(asset_id, amount, None).call()
    /// ```
    pub fn add_custom_asset(
        mut self,
        asset_id: AssetId,
        amount: u64,
        to: Option<Bech32Address>,
    ) -> Self {
        self.contract_call.add_custom_asset(asset_id, amount, to);
        self
    }

    pub fn is_payable(&self) -> bool {
        self.contract_call.is_payable
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:

    /// ```ignore
    /// let params = TxParameters { gas_price: 100, gas_limit: 1000000 };
    /// my_contract_instance.my_method(...).tx_params(params).call()
    /// ```
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    /// Sets the call parameters for a given contract call.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// let params = CallParameters { amount: 1, asset_id: BASE_ASSET_ID };
    /// my_contract_instance.my_method(...).call_params(params).call()
    /// ```
    pub fn call_params(mut self, params: CallParameters) -> Result<Self> {
        if !self.is_payable() && params.amount > 0 {
            return Err(Error::AssetsForwardedToNonPayableMethod);
        }
        self.contract_call.call_parameters = params;
        Ok(self)
    }

    /// Returns the script that executes the contract call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        build_tx_from_contract_calls(
            std::slice::from_ref(&self.contract_call),
            self.tx_parameters,
            &self.account,
        )
        .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    pub async fn submit(mut self) -> Result<ContractCallHandler<T, D>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        self.cached_tx_id = Some(provider.send_transaction(tx).await?);
        Ok(self)
    }

    pub async fn response(self) -> Result<FuelCallResponse<D>> {
        let receipts = self
            .account
            .try_provider()?
            .get_receipts(&self.cached_tx_id.expect("Cached tx_id is missing"))
            .await?;
        self.get_response(receipts)
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    ///
    pub async fn simulate(&mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate(&mut self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        self.cached_tx_id = Some(tx.id(provider.chain_id()));

        let receipts = if simulate {
            provider.checked_dry_run(tx).await?
        } else {
            let tx_id = provider.send_transaction(tx).await?;
            provider.get_receipts(&tx_id).await?
        };

        self.get_response(receipts)
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        let transaction_cost = provider
            .estimate_transaction_cost(script, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        let token = ReceiptParser::new(&receipts).parse(
            Some(&self.contract_call.contract_id),
            &self.contract_call.output_param,
        )?;
        Ok(FuelCallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        ))
    }
}

#[async_trait::async_trait]
impl<T, D> TxDependencyExtension for ContractCallHandler<T, D>
where
    T: Account,
    D: Tokenizable + Parameterize + Debug + Send + Sync,
{
    async fn simulate(&mut self) -> Result<()> {
        self.simulate().await?;
        Ok(())
    }

    fn append_variable_outputs(mut self, num: u64) -> Self {
        self.contract_call.append_variable_outputs(num);
        self
    }

    fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.contract_call.append_external_contracts(contract_id);
        self
    }
}

/// Creates an ABI call based on a function [selector](Selector) and
/// the encoding of its call arguments, which is a slice of [`Token`]s.
/// It returns a prepared [`ContractCall`] that can further be used to
/// make the actual transaction.
/// This method is the underlying implementation of the functions
/// generated from an ABI JSON spec, i.e, this is what's generated:
///
/// ```ignore
/// quote! {
///     #doc
///     pub fn #name(&self #input) -> #result {
///         contract::method_hash(#tokenized_signature, #arg)
///     }
/// }
/// ```
///
/// For more details see `code_gen` in `fuels-core`.
///
/// Note that this needs an account because the contract instance needs an account for the calls
pub fn method_hash<D: Tokenizable + Parameterize + Debug, T: Account>(
    contract_id: Bech32ContractId,
    account: T,
    signature: Selector,
    args: &[Token],
    log_decoder: LogDecoder,
    is_payable: bool,
) -> Result<ContractCallHandler<T, D>> {
    let encoded_selector = signature;

    let tx_parameters = TxParameters::default();
    let call_parameters = CallParameters::default();

    let compute_custom_input_offset = should_compute_custom_input_offset(args);

    let unresolved_bytes = ABIEncoder::encode(args)?;
    let contract_call = ContractCall {
        contract_id,
        encoded_selector,
        encoded_args: unresolved_bytes,
        call_parameters,
        compute_custom_input_offset,
        variable_outputs: vec![],
        external_contracts: vec![],
        output_param: D::param_type(),
        is_payable,
        custom_assets: Default::default(),
    };

    Ok(ContractCallHandler {
        contract_call,
        tx_parameters,
        cached_tx_id: None,
        account,
        datatype: PhantomData,
        log_decoder,
    })
}

// If the data passed into the contract method is an integer or a
// boolean, then the data itself should be passed. Otherwise, it
// should simply pass a pointer to the data in memory.
fn should_compute_custom_input_offset(args: &[Token]) -> bool {
    args.len() > 1
        || args.iter().any(|t| {
            matches!(
                t,
                Token::Array(_)
                    | Token::B256(_)
                    | Token::Bytes(_)
                    | Token::Enum(_)
                    | Token::RawSlice(_)
                    | Token::Struct(_)
                    | Token::Tuple(_)
                    | Token::U128(_)
                    | Token::U256(_)
                    | Token::Vector(_)
                    | Token::StringArray(_)
                    | Token::StringSlice(_)
                    | Token::String(_)
            )
        })
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles bundling multiple calls into a single transaction
pub struct MultiContractCallHandler<T: Account> {
    pub contract_calls: Vec<ContractCall>,
    pub log_decoder: LogDecoder,
    pub tx_parameters: TxParameters,
    // Initially `None`, gets set to the right tx id after the transaction is submitted
    cached_tx_id: Option<Bytes32>,
    pub account: T,
}

impl<T: Account> MultiContractCallHandler<T> {
    pub fn new(account: T) -> Self {
        Self {
            contract_calls: vec![],
            tx_parameters: TxParameters::default(),
            cached_tx_id: None,
            account,
            log_decoder: LogDecoder {
                log_formatters: Default::default(),
            },
        }
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call(
        &mut self,
        call_handler: ContractCallHandler<impl Account, impl Tokenizable>,
    ) -> &mut Self {
        self.log_decoder.merge(call_handler.log_decoder);
        self.contract_calls.push(call_handler.contract_call);
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    fn validate_contract_calls(&self) -> Result<()> {
        if self.contract_calls.is_empty() {
            return Err(error!(
                InvalidData,
                "No calls added. Have you used '.add_calls()'?"
            ));
        }

        let number_of_heap_type_calls = self
            .contract_calls
            .iter()
            .filter(|cc| cc.output_param.is_vm_heap_type())
            .count();

        match number_of_heap_type_calls {
            0 => Ok(()),
            1 => {
                if self
                    .contract_calls
                    .last()
                    .expect("is not empty")
                    .output_param
                    .is_vm_heap_type()
                {
                    Ok(())
                } else {
                    Err(error!(
                        InvalidData,
                        "The contract call with the heap type return must be at the last position"
                    ))
                }
            }
            _ => Err(error!(
                InvalidData,
                "`MultiContractCallHandler` can have only one call that returns a heap type"
            )),
        }
    }

    /// Returns the script that executes the contract calls
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        self.validate_contract_calls()?;

        build_tx_from_contract_calls(&self.contract_calls, self.tx_parameters, &self.account).await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    pub async fn submit(mut self) -> Result<MultiContractCallHandler<T>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        self.cached_tx_id = Some(provider.send_transaction(tx).await?);

        Ok(self)
    }

    pub async fn response<D: Tokenizable + Debug>(self) -> Result<FuelCallResponse<D>> {
        let receipts = self
            .account
            .try_provider()?
            .get_receipts(&self.cached_tx_id.expect("Cached tx_id is missing"))
            .await?;
        self.get_response(receipts)
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&mut self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &mut self,
        simulate: bool,
    ) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;
        let provider = self.account.try_provider()?;

        self.cached_tx_id = Some(tx.id(provider.chain_id()));

        let receipts = if simulate {
            provider.checked_dry_run(tx).await?
        } else {
            let tx_id = provider.send_transaction(tx).await?;
            provider.get_receipts(&tx_id).await?
        };

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let provider = self.account.try_provider()?;
        let tx = self.build_tx().await?;

        provider.checked_dry_run(tx).await?;

        Ok(())
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.build_tx().await?;

        let transaction_cost = self
            .account
            .try_provider()?
            .estimate_transaction_cost(script, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<FuelCallResponse<D>> {
        let mut receipt_parser = ReceiptParser::new(&receipts);

        let final_tokens = self
            .contract_calls
            .iter()
            .map(|call| receipt_parser.parse(Some(&call.contract_id), &call.output_param))
            .collect::<Result<Vec<_>>>()?;

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = FuelCallResponse::<D>::new(
            D::from_token(tokens_as_tuple)?,
            receipts,
            self.log_decoder.clone(),
            self.cached_tx_id,
        );

        Ok(response)
    }
}

#[async_trait::async_trait]
impl<T> TxDependencyExtension for MultiContractCallHandler<T>
where
    T: Account,
{
    async fn simulate(&mut self) -> Result<()> {
        self.simulate_without_decode().await?;
        Ok(())
    }

    fn append_variable_outputs(mut self, num: u64) -> Self {
        self.contract_calls
            .iter_mut()
            .take(1)
            .for_each(|call| call.append_variable_outputs(num));

        self
    }

    fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.contract_calls
            .iter_mut()
            .take(1)
            .for_each(|call| call.append_external_contracts(contract_id.clone()));
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn merging_overrides_storage_slots() {
        // given
        let make_slot = |id, value| StorageSlot::new([id; 32].into(), [value; 32].into());

        let slots = (1..3).map(|id| make_slot(id, 100));
        let original_config = StorageConfiguration::new(false, slots);

        let overlapping_slots = (2..4).map(|id| make_slot(id, 200));

        // when
        let original_config = original_config.add_slot_overrides(overlapping_slots);

        // then
        assert_eq!(
            HashSet::from_iter(original_config.slot_overrides.into_iter()),
            HashSet::from([make_slot(1, 100), make_slot(2, 200), make_slot(3, 200)])
        );
    }

    #[test]
    fn autoload_storage_slots() {
        // given
        let temp_dir = tempdir().unwrap();
        let contract_bin = temp_dir.path().join("my_contract.bin");
        std::fs::write(&contract_bin, "").unwrap();

        let storage_file = temp_dir.path().join("my_contract-storage_slots.json");

        let expected_storage_slots = vec![StorageSlot::new([1; 32].into(), [2; 32].into())];
        save_slots(&expected_storage_slots, &storage_file);

        let storage_config = StorageConfiguration::new(true, vec![]);
        let load_config = LoadConfiguration::default().with_storage_configuration(storage_config);

        // when
        let loaded_contract = Contract::load_from(&contract_bin, load_config).unwrap();

        // then
        assert_eq!(loaded_contract.storage_slots, expected_storage_slots);
    }

    #[test]
    fn autoload_fails_if_file_missing() {
        // given
        let temp_dir = tempdir().unwrap();
        let contract_bin = temp_dir.path().join("my_contract.bin");
        std::fs::write(&contract_bin, "").unwrap();

        let storage_config = StorageConfiguration::new(true, vec![]);
        let load_config = LoadConfiguration::default().with_storage_configuration(storage_config);

        // when
        let error = Contract::load_from(&contract_bin, load_config)
            .expect_err("Should have failed because the storage slots file is missing");

        // then
        let storage_slots_path = temp_dir.path().join("my_contract-storage_slots.json");
        let Error::InvalidData(msg) = error else {
            panic!("Expected an error of type InvalidData");
        };
        assert_eq!(msg, format!("Could not autoload storage slots from file: {storage_slots_path:?}. Either provide the file or disable autoloading in StorageConfiguration"));
    }

    fn save_slots(slots: &Vec<StorageSlot>, path: &Path) {
        std::fs::write(
            path,
            serde_json::to_string::<Vec<StorageSlot>>(slots).unwrap(),
        )
        .unwrap()
    }
}
