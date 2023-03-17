use std::{collections::HashMap, fmt::Debug, fs, marker::PhantomData, panic, path::Path};

use fuel_abi_types::error_codes::{FAILED_SEND_MESSAGE_SIGNAL, FAILED_TRANSFER_TO_ADDRESS_SIGNAL};
use fuel_tx::{
    Address, AssetId, Bytes32, Contract as FuelContract, ContractId, Output, Receipt, Salt,
    StorageSlot,
};
use fuel_vm::fuel_asm::PanicReason;
use fuels_core::{
    abi_decoder::ABIDecoder,
    abi_encoder::{ABIEncoder, UnresolvedBytes},
};
use fuels_signers::{
    provider::{Provider, TransactionCost},
    Signer, WalletUnlocked,
};
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId},
    constants::{BASE_ASSET_ID, DEFAULT_CALL_PARAMS_AMOUNT},
    errors::{error, Error, Result},
    param_types::{ParamType, ReturnLocation},
    traits::{Parameterize, Tokenizable},
    transaction::{CreateTransaction, ScriptTransaction, Transaction, TxParameters},
    Selector, Token,
};
use itertools::Itertools;

use crate::{
    call_response::FuelCallResponse,
    call_utils::{build_tx_from_contract_calls, simulate_and_check_success},
    logs::{map_revert_error, LogDecoder},
    Configurables,
};

#[derive(Debug)]
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

    pub fn set_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn set_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = asset_id;
        self
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    pub fn set_gas_forwarded(mut self, gas_forwarded: u64) -> Self {
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

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

// Trait implemented by contract instances so that
// they can be passed to the `set_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

/// A compiled representation of a contract
#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    binary: Vec<u8>,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
}

/// Configuration for contract storage
#[derive(Debug, Clone, Default)]
pub struct StorageConfiguration {
    storage_path: String,
    manual_storage: Vec<StorageSlot>,
}

impl StorageConfiguration {
    pub fn new(storage_path: String, manual_storage: Vec<StorageSlot>) -> Self {
        Self {
            storage_path,
            manual_storage,
        }
    }

    pub fn set_storage_path(mut self, storage_path: String) -> Self {
        self.storage_path = storage_path;
        self
    }

    pub fn set_manual_storage(mut self, manual_storage: Vec<StorageSlot>) -> Self {
        self.manual_storage = manual_storage;
        self
    }
}

/// Configuration for contract deployment
#[derive(Debug, Clone, Default)]
pub struct DeployConfiguration {
    tx_parameters: TxParameters,
    storage: StorageConfiguration,
    configurables: Configurables,
    salt: Salt,
}

impl DeployConfiguration {
    pub fn new(
        tx_parameters: TxParameters,
        storage: StorageConfiguration,
        configurables: impl Into<Configurables>,
        salt: impl Into<Salt>,
    ) -> Self {
        Self {
            tx_parameters,
            storage,
            configurables: configurables.into(),
            salt: salt.into(),
        }
    }

    pub fn set_tx_parameters(mut self, tx_parameters: TxParameters) -> Self {
        self.tx_parameters = tx_parameters;
        self
    }

    pub fn set_storage_configuration(mut self, storage: StorageConfiguration) -> Self {
        self.storage = storage;
        self
    }

    pub fn set_configurables(mut self, configurables: impl Into<Configurables>) -> Self {
        self.configurables = configurables.into();
        self
    }

    pub fn set_salt(mut self, salt: impl Into<Salt>) -> Self {
        self.salt = salt.into();
        self
    }
}

/// [`Contract`] is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
/// The contract has a wallet attribute, used to pay for transactions and sign them.
/// It allows doing calls without passing a wallet/signer each time.
pub struct Contract {
    pub compiled_contract: CompiledContract,
    pub wallet: WalletUnlocked,
}

impl Contract {
    pub fn new(compiled_contract: CompiledContract, wallet: WalletUnlocked) -> Self {
        Self {
            compiled_contract,
            wallet,
        }
    }

    pub fn compute_contract_id_and_state_root(
        compiled_contract: &CompiledContract,
    ) -> (ContractId, Bytes32) {
        let fuel_contract = FuelContract::from(compiled_contract.binary.as_slice());
        let root = fuel_contract.root();
        let state_root = FuelContract::initial_state_root(compiled_contract.storage_slots.iter());

        let contract_id = fuel_contract.id(&compiled_contract.salt, &root, &state_root);

        (contract_id, state_root)
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
    ///         Contract::method_hash(#tokenized_signature, #arg)
    ///     }
    /// }
    /// ```
    ///
    /// For more details see `code_gen` in `fuels-core`.
    ///
    /// Note that this needs a wallet because the contract instance needs a wallet for the calls
    pub fn method_hash<D: Tokenizable + Parameterize + Debug>(
        provider: &Provider,
        contract_id: Bech32ContractId,
        wallet: &WalletUnlocked,
        signature: Selector,
        args: &[Token],
        log_decoder: LogDecoder,
        is_payable: bool,
    ) -> Result<ContractCallHandler<D>> {
        let encoded_selector = signature;

        let tx_parameters = TxParameters::default();
        let call_parameters = CallParameters::default();

        let compute_custom_input_offset = Self::should_compute_custom_input_offset(args);

        let unresolved_bytes = ABIEncoder::encode(args)?;
        let contract_call = ContractCall {
            contract_id,
            encoded_selector,
            encoded_args: unresolved_bytes,
            call_parameters,
            compute_custom_input_offset,
            variable_outputs: None,
            message_outputs: None,
            external_contracts: vec![],
            output_param: D::param_type(),
            is_payable,
            custom_assets: Default::default(),
        };

        Ok(ContractCallHandler {
            contract_call,
            tx_parameters,
            wallet: wallet.clone(),
            provider: provider.clone(),
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
                    Token::String(_)
                        | Token::Struct(_)
                        | Token::Enum(_)
                        | Token::B256(_)
                        | Token::Tuple(_)
                        | Token::Array(_)
                        | Token::Vector(_)
                )
            })
    }

    /// Loads a compiled contract and deploys it to a running node
    pub async fn deploy(
        binary_filepath: &str,
        wallet: &WalletUnlocked,
        configuration: DeployConfiguration,
    ) -> Result<Bech32ContractId> {
        let tx_parameters = configuration.tx_parameters;
        let compiled_contract = Self::load_contract(binary_filepath, configuration)?;

        Self::deploy_loaded(compiled_contract, wallet, tx_parameters).await
    }

    /// Deploys a compiled contract to a running node
    /// To deploy a contract, you need a wallet with enough assets to pay for deployment. This
    /// wallet will also receive the change.
    async fn deploy_loaded(
        compiled_contract: CompiledContract,
        wallet: &WalletUnlocked,
        params: TxParameters,
    ) -> Result<Bech32ContractId> {
        let (mut tx, contract_id) =
            Self::contract_deployment_transaction(compiled_contract, params);

        // The first witness is the bytecode we're deploying.
        // The signature will be appended at position 1 of
        // the witness list
        wallet.add_fee_resources(&mut tx, 0, 1).await?;
        wallet.sign_transaction(&mut tx).await?;

        let provider = wallet.get_provider()?;
        let chain_info = provider.chain_info().await?;

        tx.check_without_signatures(
            chain_info.latest_block.header.height,
            &chain_info.consensus_parameters,
        )?;
        provider.send_transaction(&tx).await?;

        Ok(contract_id)
    }

    pub fn load_contract(
        binary_filepath: &str,
        configuration: DeployConfiguration,
    ) -> Result<CompiledContract> {
        Self::validate_path_and_extension(binary_filepath, "bin")?;

        let mut binary = fs::read(binary_filepath)
            .map_err(|_| error!(InvalidData, "failed to read binary: '{binary_filepath}'"))?;

        configuration.configurables.update_constants_in(&mut binary);

        let storage_slots = Self::get_storage_slots(configuration.storage)?;

        Ok(CompiledContract {
            binary,
            salt: configuration.salt,
            storage_slots,
        })
    }

    fn validate_path_and_extension(file_path: &str, extension: &str) -> Result<()> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(error!(InvalidData, "file '{file_path}' does not exist"));
        }

        let path_extension = path
            .extension()
            .ok_or_else(|| error!(InvalidData, "could not extract extension from: {file_path}"))?;

        if extension != path_extension {
            return Err(error!(
                InvalidData,
                "expected `{file_path}` to have '.{extension}' extension"
            ));
        }

        Ok(())
    }

    /// Crafts a transaction used to deploy a contract
    fn contract_deployment_transaction(
        compiled_contract: CompiledContract,
        params: TxParameters,
    ) -> (CreateTransaction, Bech32ContractId) {
        let (contract_id, state_root) =
            Self::compute_contract_id_and_state_root(&compiled_contract);

        let bytecode_witness_index = 0;
        let outputs = vec![Output::contract_created(contract_id, state_root)];
        let witnesses = vec![compiled_contract.binary.into()];

        let tx = CreateTransaction::build_contract_deployment_tx(
            bytecode_witness_index,
            outputs,
            witnesses,
            compiled_contract.salt,
            compiled_contract.storage_slots,
            params,
        );

        (tx, contract_id.into())
    }

    fn get_storage_slots(configuration: StorageConfiguration) -> Result<Vec<StorageSlot>> {
        let StorageConfiguration {
            storage_path,
            manual_storage,
        } = configuration;

        if storage_path.is_empty() {
            return Ok(manual_storage);
        }

        Self::validate_path_and_extension(&storage_path, "json")?;

        let storage_json_string = fs::read_to_string(&storage_path).map_err(|_| {
            error!(
                InvalidData,
                "failed to read storage configuration from: '{storage_path}'"
            )
        })?;

        let storage_slots: Vec<StorageSlot> = serde_json::from_str(&storage_json_string)?;

        Ok(manual_storage
            .into_iter()
            .chain(storage_slots.into_iter())
            .unique()
            .collect())
    }
}

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: Bech32ContractId,
    pub encoded_args: UnresolvedBytes,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub compute_custom_input_offset: bool,
    pub variable_outputs: Option<Vec<Output>>,
    pub message_outputs: Option<Vec<Output>>,
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
            variable_outputs: Some(variable_outputs),
            ..self
        }
    }

    pub fn with_message_outputs(self, message_outputs: Vec<Output>) -> ContractCall {
        ContractCall {
            message_outputs: Some(message_outputs),
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
        let new_variable_outputs = vec![
            Output::Variable {
                amount: 0,
                to: Address::zeroed(),
                asset_id: AssetId::default(),
            };
            num as usize
        ];

        match self.variable_outputs {
            Some(ref mut outputs) => outputs.extend(new_variable_outputs),
            None => self.variable_outputs = Some(new_variable_outputs),
        }
    }

    pub fn append_external_contracts(&mut self, contract_id: Bech32ContractId) {
        self.external_contracts.push(contract_id)
    }

    pub fn append_message_outputs(&mut self, num: u64) {
        let new_message_outputs = vec![
            Output::Message {
                recipient: Address::zeroed(),
                amount: 0,
            };
            num as usize
        ];

        match self.message_outputs {
            Some(ref mut outputs) => outputs.extend(new_message_outputs),
            None => self.message_outputs = Some(new_message_outputs),
        }
    }

    fn is_missing_output_variables(receipts: &[Receipt]) -> bool {
        receipts.iter().any(
            |r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_TRANSFER_TO_ADDRESS_SIGNAL),
        )
    }

    fn is_missing_message_output(receipts: &[Receipt]) -> bool {
        receipts
            .iter()
            .any(|r| matches!(r, Receipt::Revert { ra, .. } if *ra == FAILED_SEND_MESSAGE_SIGNAL))
    }

    fn find_contract_not_in_inputs(receipts: &[Receipt]) -> Option<&Receipt> {
        receipts.iter().find(
            |r| matches!(r, Receipt::Panic { reason, .. } if *reason.reason() == PanicReason::ContractNotInInputs ),
        )
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Bech32Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }
}

/// Based on the receipts returned by the call, the contract ID (which is null in the case of a
/// script), and the output param, decode the values and return them.
pub fn get_decoded_output(
    receipts: &[Receipt],
    contract_id: Option<&Bech32ContractId>,
    output_param: &ParamType,
) -> Result<Token> {
    let null_contract_id = ContractId::new([0u8; 32]);
    // Multiple returns are handled as one `Tuple` (which has its own `ParamType`)
    let contract_id: ContractId = match contract_id {
        Some(contract_id) => contract_id.into(),
        // During a script execution, the script's contract id is the **null** contract id
        None => null_contract_id,
    };
    let encoded_value = match output_param.get_return_location() {
        ReturnLocation::ReturnData if output_param.is_vector() => {
            // If the output of the function is a vector, then there are 2 consecutive ReturnData
            // receipts. The first one is the one that returns the pointer to the vec struct in the
            // VM memory, the second one contains the actual vector bytes (that the previous receipt
            // points to).
            // We ensure to take the right "first" ReturnData receipt by checking for the
            // contract_id. There are no receipts in between the two ReturnData receipts because of
            // the way the scripts are built (the calling script adds a RETD just after the CALL
            // opcode, see `get_single_call_instructions`).
            let vector_data = receipts
                .iter()
                .tuple_windows()
                .find_map(|(current_receipt, next_receipt)| {
                    extract_vec_data(current_receipt, next_receipt, contract_id)
                })
                .cloned()
                .expect("Could not extract vector data");
            Some(vector_data)
        }
        ReturnLocation::ReturnData => receipts
            .iter()
            .find(|receipt| {
                matches!(receipt,
                    Receipt::ReturnData { id, data, .. } if *id == contract_id && !data.is_empty())
            })
            .map(|receipt| {
                receipt
                    .data()
                    .expect("ReturnData should have data")
                    .to_vec()
            }),
        ReturnLocation::Return => receipts
            .iter()
            .find(|receipt| {
                matches!(receipt,
                    Receipt::Return { id, ..} if *id == contract_id)
            })
            .map(|receipt| {
                receipt
                    .val()
                    .expect("Return should have val")
                    .to_be_bytes()
                    .to_vec()
            }),
    }
    .unwrap_or_default();

    let decoded_value = ABIDecoder::decode_single(output_param, &encoded_value)?;
    Ok(decoded_value)
}

fn extract_vec_data<'a>(
    current_receipt: &Receipt,
    next_receipt: &'a Receipt,
    contract_id: ContractId,
) -> Option<&'a Vec<u8>> {
    match (current_receipt, next_receipt) {
        (
            Receipt::ReturnData {
                id: first_id,
                data: first_data,
                ..
            },
            Receipt::ReturnData {
                id: second_id,
                data: vec_data,
                ..
            },
        ) if *first_id == contract_id
            && !first_data.is_empty()
            && *second_id == ContractId::zeroed() =>
        {
            Some(vec_data)
        }
        _ => None,
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct ContractCallHandler<D> {
    pub contract_call: ContractCall,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
    pub provider: Provider,
    pub datatype: PhantomData<D>,
    pub log_decoder: LogDecoder,
}

impl<D> ContractCallHandler<D>
where
    D: Tokenizable + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).set_contract_ids(&[another_contract_id]).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn set_contract_ids(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.contract_call.external_contracts = contract_ids.to_vec();
        self
    }

    /// Sets external contract instances as dependencies to this contract's call.
    /// Effectively, this will be used to: merge `LogDecoder`s and create
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`] pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).set_contracts(&[another_contract_instance]).call()
    /// ```
    pub fn set_contracts(mut self, contracts: &[&dyn SettableContract]) -> Self {
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
    /// - `address`: The optional wallet address that the output amount will be sent to. If not provided, the asset will be sent to the users wallet address.
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

    /// Appends additional external contracts as dependencies to this contract's
    /// call. Effectively, this will be used to create additional
    /// [`fuel_tx::Input::Contract`]/[`fuel_tx::Output::Contract`]
    /// pairs and set them into the transaction. Note that this is a builder
    /// method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).append_contracts(additional_contract_id).call()
    /// ```
    ///
    /// [`Input::Contract`]: fuel_tx::Input::Contract
    /// [`Output::Contract`]: fuel_tx::Output::Contract
    pub fn append_contract(mut self, contract_id: Bech32ContractId) -> Self {
        self.contract_call.append_external_contracts(contract_id);
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

    /// Appends `num` [`fuel_tx::Output::Variable`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).add_variable_outputs(num).call()
    /// ```
    ///
    /// [`Output::Variable`]: fuel_tx::Output::Variable
    pub fn append_variable_outputs(mut self, num: u64) -> Self {
        self.contract_call.append_variable_outputs(num);
        self
    }

    /// Appends `num` [`fuel_tx::Output::Message`]s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    ///
    /// ```ignore
    /// my_contract_instance.my_method(...).add_message_outputs(num).call()
    /// ```
    ///
    /// [`Output::Message`]: fuel_tx::Output::Message
    pub fn append_message_outputs(mut self, num: u64) -> Self {
        self.contract_call.append_message_outputs(num);
        self
    }

    /// Returns the script that executes the contract call
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        build_tx_from_contract_calls(
            std::slice::from_ref(&self.contract_call),
            self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    ///
    pub async fn simulate(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate(&self, simulate: bool) -> Result<FuelCallResponse<D>> {
        let tx = self.build_tx().await?;

        let receipts = if simulate {
            simulate_and_check_success(&self.provider, &tx).await?
        } else {
            self.provider.send_transaction(&tx).await?
        };

        self.get_response(receipts)
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            match self.simulate().await {
                Ok(_) => return Ok(self),

                Err(Error::RevertTransactionError { ref receipts, .. }) => {
                    self = self.append_missing_deps(receipts);
                }

                Err(other_error) => return Err(other_error),
            }
        }

        self.simulate().await.map(|_| self)
    }

    fn append_missing_deps(mut self, receipts: &[Receipt]) -> Self {
        if ContractCall::is_missing_output_variables(receipts) {
            self = self.append_variable_outputs(1)
        }
        if ContractCall::is_missing_message_output(receipts) {
            self = self.append_message_outputs(1);
        }
        if let Some(panic_receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
            let contract_id = Bech32ContractId::from(
                *panic_receipt
                    .contract_id()
                    .expect("Panic receipt must contain contract id."),
            );
            self = self.append_contract(contract_id);
        }

        self
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.build_tx().await?;

        let transaction_cost = self
            .provider
            .estimate_transaction_cost(&script, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>> {
        let token = get_decoded_output(
            &receipts,
            Some(&self.contract_call.contract_id),
            &self.contract_call.output_param,
        )?;
        Ok(FuelCallResponse::new(
            D::from_token(token)?,
            receipts,
            self.log_decoder.clone(),
        ))
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles bundling multiple calls into a single transaction
pub struct MultiContractCallHandler {
    pub contract_calls: Vec<ContractCall>,
    pub log_decoder: LogDecoder,
    pub tx_parameters: TxParameters,
    pub wallet: WalletUnlocked,
}

impl MultiContractCallHandler {
    pub fn new(wallet: WalletUnlocked) -> Self {
        Self {
            contract_calls: vec![],
            tx_parameters: TxParameters::default(),
            wallet,
            log_decoder: LogDecoder {
                type_lookup: HashMap::new(),
            },
        }
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call<D: Tokenizable>(&mut self, call_handler: ContractCallHandler<D>) -> &mut Self {
        self.log_decoder.merge(call_handler.log_decoder);
        self.contract_calls.push(call_handler.contract_call);
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method
    pub fn tx_params(&mut self, params: TxParameters) -> &mut Self {
        self.tx_parameters = params;
        self
    }

    /// Returns the script that executes the contract calls
    pub async fn build_tx(&self) -> Result<ScriptTransaction> {
        if self.contract_calls.is_empty() {
            panic!("No calls added. Have you used '.add_calls()'?");
        }

        build_tx_from_contract_calls(&self.contract_calls, self.tx_parameters, &self.wallet).await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(false)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>> {
        self.call_or_simulate(true)
            .await
            .map_err(|err| map_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &self,
        simulate: bool,
    ) -> Result<FuelCallResponse<D>> {
        let provider = self.wallet.get_provider()?;
        let tx = self.build_tx().await?;

        let receipts = if simulate {
            simulate_and_check_success(provider, &tx).await?
        } else {
            provider.send_transaction(&tx).await?
        };

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<()> {
        let provider = self.wallet.get_provider()?;
        let tx = self.build_tx().await?;

        simulate_and_check_success(provider, &tx).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(mut self, max_attempts: Option<u64>) -> Result<Self> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            match self.simulate_without_decode().await {
                Ok(_) => return Ok(self),

                Err(Error::RevertTransactionError { ref receipts, .. }) => {
                    self = self.append_missing_dependencies(receipts);
                }

                Err(other_error) => return Err(other_error),
            }
        }

        Ok(self)
    }

    fn append_missing_dependencies(mut self, receipts: &[Receipt]) -> Self {
        // Append to any call, they will be merged to a single script tx
        // At least 1 call should exist at this point, otherwise simulate would have failed
        if ContractCall::is_missing_output_variables(receipts) {
            self.contract_calls
                .iter_mut()
                .take(1)
                .for_each(|call| call.append_variable_outputs(1));
        }

        if ContractCall::is_missing_message_output(receipts) {
            self.contract_calls
                .iter_mut()
                .take(1)
                .for_each(|call| call.append_message_outputs(1));
        }

        if let Some(panic_receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
            let contract_id = Bech32ContractId::from(
                *panic_receipt
                    .contract_id()
                    .expect("Panic receipt must contain contract id."),
            );
            self.contract_calls
                .iter_mut()
                .take(1)
                .for_each(|call| call.append_external_contracts(contract_id.clone()));
        }

        self
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost> {
        let script = self.build_tx().await?;

        let transaction_cost = self
            .wallet
            .get_provider()?
            .estimate_transaction_cost(&script, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<FuelCallResponse<D>> {
        let mut final_tokens = vec![];

        for call in self.contract_calls.iter() {
            let decoded =
                get_decoded_output(&receipts, Some(&call.contract_id), &call.output_param)?;

            final_tokens.push(decoded.clone());
        }

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = FuelCallResponse::<D>::new(
            D::from_token(tokens_as_tuple)?,
            receipts,
            self.log_decoder.clone(),
        );

        Ok(response)
    }
}
