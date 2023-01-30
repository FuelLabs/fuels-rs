use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs,
    marker::PhantomData,
    panic,
    path::Path,
    str::FromStr,
};

use fuel_tx::{
    Address, AssetId, Bytes32, Checkable, Contract as FuelContract, ContractId, Create, Output,
    Receipt, Salt, StorageSlot, Transaction,
};
use fuel_vm::fuel_asm::PanicReason;
use fuels_core::{
    abi_decoder::ABIDecoder,
    abi_encoder::{ABIEncoder, UnresolvedBytes},
    constants::FAILED_TRANSFER_TO_ADDRESS_SIGNAL,
    parameters::{CallParameters, StorageConfiguration, TxParameters},
    traits::{Parameterize, Tokenizable},
};
use fuels_signers::{
    provider::{Provider, TransactionCost},
    Signer, WalletUnlocked,
};
use fuels_types::{
    bech32::{Bech32Address, Bech32ContractId},
    core::{Selector, Token},
    errors::Error,
    param_types::{ParamType, ReturnLocation},
};

use crate::{
    call_response::FuelCallResponse,
    execution_script::ExecutableFuelCall,
    logs::{decode_revert_error, LogDecoder},
};

/// How many times to attempt to resolve missing tx dependencies.
pub const DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS: u64 = 10;

// Trait implemented by contract instances so that
// they can be passed to the `set_contracts` method
pub trait SettableContract {
    fn id(&self) -> Bech32ContractId;
    fn log_decoder(&self) -> LogDecoder;
}

/// A compiled representation of a contract.
#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    pub raw: Vec<u8>,
    pub salt: Salt,
    pub storage_slots: Vec<StorageSlot>,
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
        let fuel_contract = FuelContract::from(compiled_contract.raw.clone());
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
    ) -> Result<ContractCallHandler<D>, Error> {
        let encoded_selector = signature;

        let tx_parameters = TxParameters::default();
        let call_parameters = CallParameters::default();

        let compute_custom_input_offset = Contract::should_compute_custom_input_offset(args);

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
                        | Token::Byte(_)
                        | Token::Vector(_)
                )
            })
    }

    /// Loads a compiled contract and deploys it to a running node
    pub async fn deploy(
        binary_filepath: &str,
        wallet: &WalletUnlocked,
        params: TxParameters,
        storage_configuration: StorageConfiguration,
    ) -> Result<Bech32ContractId, Error> {
        let mut compiled_contract =
            Contract::load_contract(binary_filepath, &storage_configuration.storage_path)?;

        Self::merge_storage_vectors(&storage_configuration, &mut compiled_contract);

        Self::deploy_loaded(&(compiled_contract), wallet, params).await
    }

    /// Loads a compiled contract with salt and deploys it to a running node
    pub async fn deploy_with_parameters(
        binary_filepath: &str,
        wallet: &WalletUnlocked,
        params: TxParameters,
        storage_configuration: StorageConfiguration,
        salt: Salt,
    ) -> Result<Bech32ContractId, Error> {
        let mut compiled_contract = Contract::load_contract_with_parameters(
            binary_filepath,
            &storage_configuration.storage_path,
            salt,
        )?;

        Self::merge_storage_vectors(&storage_configuration, &mut compiled_contract);

        Self::deploy_loaded(&(compiled_contract), wallet, params).await
    }

    fn merge_storage_vectors(
        storage_configuration: &StorageConfiguration,
        compiled_contract: &mut CompiledContract,
    ) {
        match &storage_configuration.manual_storage_vec {
            Some(storage) if !storage.is_empty() => {
                compiled_contract.storage_slots =
                    Self::merge_storage_slots(storage, &compiled_contract.storage_slots);
            }
            _ => {}
        }
    }

    /// Deploys a compiled contract to a running node
    /// To deploy a contract, you need a wallet with enough assets to pay for deployment. This
    /// wallet will also receive the change.
    pub async fn deploy_loaded(
        compiled_contract: &CompiledContract,
        wallet: &WalletUnlocked,
        params: TxParameters,
    ) -> Result<Bech32ContractId, Error> {
        let (mut tx, contract_id) =
            Self::contract_deployment_transaction(compiled_contract, params).await?;

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
        storage_path: &Option<String>,
    ) -> Result<CompiledContract, Error> {
        Self::load_contract_with_parameters(binary_filepath, storage_path, Salt::from([0u8; 32]))
    }

    pub fn load_contract_with_parameters(
        binary_filepath: &str,
        storage_path: &Option<String>,
        salt: Salt,
    ) -> Result<CompiledContract, Error> {
        let extension = Path::new(binary_filepath)
            .extension()
            .expect("Could not extract extension from file path");
        if extension != "bin" {
            return Err(Error::InvalidData(format!(
                "The file extension '{}' is not recognized. Did you mean '.bin'?",
                extension
                    .to_str()
                    .expect("Could not convert extension to &str")
            )));
        }

        let bin = std::fs::read(binary_filepath).map_err(|_| {
            Error::InvalidData(format!(
                "Failed to read binary file with path '{}'",
                &binary_filepath
            ))
        })?;

        let storage = match storage_path {
            Some(path) if Path::new(&path).exists() => Self::get_storage_vec(path),
            Some(path) if !Path::new(&path).exists() => {
                return Err(Error::InvalidData(path.to_owned()));
            }
            _ => {
                vec![]
            }
        };

        Ok(CompiledContract {
            raw: bin,
            salt,
            storage_slots: storage,
        })
    }

    fn merge_storage_slots(
        manual_storage: &[StorageSlot],
        contract_storage: &[StorageSlot],
    ) -> Vec<StorageSlot> {
        let mut return_storage: Vec<StorageSlot> = manual_storage.to_owned();
        let keys: HashSet<Bytes32> = manual_storage.iter().map(|slot| *slot.key()).collect();

        contract_storage.iter().for_each(|slot| {
            if !keys.contains(slot.key()) {
                return_storage.push(slot.clone())
            }
        });

        return_storage
    }

    /// Crafts a transaction used to deploy a contract
    pub async fn contract_deployment_transaction(
        compiled_contract: &CompiledContract,
        params: TxParameters,
    ) -> Result<(Create, Bech32ContractId), Error> {
        let bytecode_witness_index = 0;
        let storage_slots: Vec<StorageSlot> = compiled_contract.storage_slots.clone();
        let witnesses = vec![compiled_contract.raw.clone().into()];

        let (contract_id, state_root) = Self::compute_contract_id_and_state_root(compiled_contract);

        let outputs = vec![Output::contract_created(contract_id, state_root)];

        let tx = Transaction::create(
            params.gas_price,
            params.gas_limit,
            params.maturity,
            bytecode_witness_index,
            compiled_contract.salt,
            storage_slots,
            vec![],
            outputs,
            witnesses,
        );

        Ok((tx, contract_id.into()))
    }

    fn get_storage_vec(storage_path: &str) -> Vec<StorageSlot> {
        let mut return_storage: Vec<StorageSlot> = vec![];

        let storage_json_string = fs::read_to_string(storage_path).expect("Unable to read file");

        let storage: serde_json::Value = serde_json::from_str(storage_json_string.as_str())
            .expect("JSON was not well-formatted");

        for slot in storage.as_array().unwrap() {
            return_storage.push(StorageSlot::new(
                Bytes32::from_str(slot["key"].as_str().unwrap()).unwrap(),
                Bytes32::from_str(slot["value"].as_str().unwrap()).unwrap(),
            ));
        }

        return_storage
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
) -> Result<Token, Error> {
    // Multiple returns are handled as one `Tuple` (which has its own `ParamType`)
    let contract_id: ContractId = match contract_id {
        Some(contract_id) => contract_id.into(),
        // During a script execution, the script's contract id is the **null** contract id
        None => ContractId::new([0u8; 32]),
    };
    let encoded_value = match output_param.get_return_location() {
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
    pub fn call_params(mut self, params: CallParameters) -> Self {
        self.contract_call.call_parameters = params;
        self
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

    /// Call a contract's method on the node. If `simulate == true`, then the call is done in a
    /// read-only manner, using a `dry-run`. The [`FuelCallResponse`] struct contains the method's
    /// value in its `value` field as an actual typed value `D` (if your method returns `bool`,
    /// it will be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of [`FuelCallResponse`], `receipts`, contains the receipts of the transaction.
    async fn call_or_simulate(&self, simulate: bool) -> Result<FuelCallResponse<D>, Error> {
        let script = self.get_executable_call().await?;

        let receipts = if simulate {
            script.simulate(&self.provider).await?
        } else {
            script.execute(&self.provider).await?
        };

        self.get_response(receipts)
    }

    /// Returns the script that executes the contract call
    pub async fn get_executable_call(&self) -> Result<ExecutableFuelCall, Error> {
        ExecutableFuelCall::from_contract_calls(
            std::slice::from_ref(&self.contract_call),
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(&self, false)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [`call`] method because the API is more user-friendly this way.
    ///
    /// [`call`]: Self::call
    pub async fn simulate(self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(&self, true)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<(), Error> {
        let script = self.get_executable_call().await?;
        let provider = self.wallet.get_provider()?;

        script.simulate(provider).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(
        mut self,
        max_attempts: Option<u64>,
    ) -> Result<Self, Error> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate_without_decode().await;

            match result {
                Err(Error::RevertTransactionError(_, receipts))
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self = self.append_variable_outputs(1);
                }

                Err(Error::RevertTransactionError(_, ref receipts)) => {
                    if let Some(receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
                        let contract_id = Bech32ContractId::from(*receipt.contract_id().unwrap());
                        self = self.append_contract(contract_id);
                    } else {
                        return Err(result.expect_err("Couldn't estimate tx dependencies because we couldn't find the missing contract input"));
                    }
                }

                Err(e) => return Err(e),
                _ => return Ok(self),
            }
        }

        // confirm if successful or propagate error
        match self.call_or_simulate(true).await {
            Ok(_) => Ok(self),
            Err(e) => Err(e),
        }
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost, Error> {
        let script = self.get_executable_call().await?;

        let transaction_cost = self
            .provider
            .estimate_transaction_cost(&script.tx, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<FuelCallResponse<D>, Error> {
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
    pub async fn get_executable_call(&self) -> Result<ExecutableFuelCall, Error> {
        if self.contract_calls.is_empty() {
            panic!("No calls added. Have you used '.add_calls()'?");
        }

        ExecutableFuelCall::from_contract_calls(
            &self.contract_calls,
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(self, false)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the [call] method because the API is more user-friendly this way.
    ///
    /// [call]: Self::call
    pub async fn simulate<D: Tokenizable + Debug>(&self) -> Result<FuelCallResponse<D>, Error> {
        Self::call_or_simulate(self, true)
            .await
            .map_err(|err| decode_revert_error(err, &self.log_decoder))
    }

    async fn call_or_simulate<D: Tokenizable + Debug>(
        &self,
        simulate: bool,
    ) -> Result<FuelCallResponse<D>, Error> {
        let script = self.get_executable_call().await?;

        let provider = self.wallet.get_provider()?;

        let receipts = if simulate {
            script.simulate(provider).await?
        } else {
            script.execute(provider).await?
        };

        self.get_response(receipts)
    }

    /// Simulates a call without needing to resolve the generic for the return type
    async fn simulate_without_decode(&self) -> Result<(), Error> {
        let script = self.get_executable_call().await?;
        let provider = self.wallet.get_provider()?;

        script.simulate(provider).await?;

        Ok(())
    }

    /// Simulates the call and attempts to resolve missing tx dependencies.
    /// Forwards the received error if it cannot be fixed.
    pub async fn estimate_tx_dependencies(
        mut self,
        max_attempts: Option<u64>,
    ) -> Result<Self, Error> {
        let attempts = max_attempts.unwrap_or(DEFAULT_TX_DEP_ESTIMATION_ATTEMPTS);

        for _ in 0..attempts {
            let result = self.simulate_without_decode().await;

            match result {
                Err(Error::RevertTransactionError(_, receipts))
                    if ContractCall::is_missing_output_variables(&receipts) =>
                {
                    self.contract_calls
                        .iter_mut()
                        .take(1)
                        .for_each(|call| call.append_variable_outputs(1));
                }

                Err(Error::RevertTransactionError(_, ref receipts)) => {
                    if let Some(receipt) = ContractCall::find_contract_not_in_inputs(receipts) {
                        let contract_id = Bech32ContractId::from(*receipt.contract_id().unwrap());
                        self.contract_calls
                            .iter_mut()
                            .take(1)
                            .for_each(|call| call.append_external_contracts(contract_id.clone()));
                    } else {
                        return Err(result.expect_err("Couldn't estimate tx dependencies because we couldn't find the missing contract input"));
                    }
                }

                Err(e) => return Err(e),
                _ => return Ok(self),
            }
        }

        Ok(self)
    }

    /// Get a contract's estimated cost
    pub async fn estimate_transaction_cost(
        &self,
        tolerance: Option<f64>,
    ) -> Result<TransactionCost, Error> {
        let script = self.get_executable_call().await?;

        let transaction_cost = self
            .wallet
            .get_provider()?
            .estimate_transaction_cost(&script.tx, tolerance)
            .await?;

        Ok(transaction_cost)
    }

    /// Create a [`FuelCallResponse`] from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        receipts: Vec<Receipt>,
    ) -> Result<FuelCallResponse<D>, Error> {
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

#[cfg(test)]
mod test {
    use fuels_test_helpers::launch_provider_and_get_wallet;

    use super::*;

    #[tokio::test]
    #[should_panic(expected = "The file extension 'json' is not recognized. Did you mean '.bin'?")]
    async fn deploy_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy(
            "tests/types/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "The file extension 'json' is not recognized. Did you mean '.bin'?")]
    async fn deploy_with_salt_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy_with_parameters(
            "tests/types/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
            Salt::default(),
        )
        .await
        .unwrap();
    }
}
