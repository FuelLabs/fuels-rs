use std::collections::HashSet;
use std::fmt::Debug;
use std::fs;
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use fuel_gql_client::{
    fuel_tx::{Contract as FuelContract, Output, Receipt, StorageSlot, Transaction},
    fuel_types::{Address, AssetId, Salt},
};

use fuels_core::abi_decoder::ABIDecoder;
use fuels_core::abi_encoder::ABIEncoder;
use fuels_core::parameters::StorageConfiguration;
use fuels_core::tx::{Bytes32, ContractId};
use fuels_core::{
    constants::{BASE_ASSET_ID, DEFAULT_SPENDABLE_COIN_AMOUNT},
    parameters::{CallParameters, TxParameters},
    Selector, Token, Tokenizable,
};
use fuels_signers::{provider::Provider, LocalWallet, Signer};
use fuels_types::bech32::Bech32ContractId;
use fuels_types::{
    errors::Error,
    param_types::{ParamType, ReturnLocation},
};

use crate::script::Script;

#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    pub raw: Vec<u8>,
    pub salt: Salt,
    pub storage_slots: Vec<StorageSlot>,
}

/// Contract is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
/// The contract has a wallet attribute, used to pay for transactions and sign them.
/// It allows doing calls without passing a wallet/signer each time.
pub struct Contract {
    pub compiled_contract: CompiledContract,
    pub wallet: LocalWallet,
}

/// CallResponse is a struct that is returned by a call to the contract. Its value field
/// holds the decoded typed value returned by the contract's method. The other field
/// holds all the receipts returned by the call.
#[derive(Debug)]
// ANCHOR: call_response
pub struct CallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub gas_used: u64,
    pub logs: Vec<String>,
}
// ANCHOR_END: call_response

impl<D> CallResponse<D> {
    /// Get all the logs from LogData receipts
    fn get_logs(receipts: &[Receipt]) -> Vec<String> {
        receipts
            .iter()
            .filter(|r| matches!(r, Receipt::LogData { .. }))
            .map(|r| hex::encode(r.data().unwrap()))
            .collect::<Vec<String>>()
    }

    /// Get the gas used from ScriptResult receipt
    fn get_gas_used(receipts: &[Receipt]) -> u64 {
        receipts
            .iter()
            .rfind(|r| matches!(r, Receipt::ScriptResult { .. }))
            .expect("could not retrieve ScriptResult")
            .gas_used()
            .expect("could not retrieve gas used")
    }

    pub fn new(value: D, receipts: Vec<Receipt>) -> Self {
        Self {
            value,
            logs: Self::get_logs(&receipts),
            gas_used: Self::get_gas_used(&receipts),
            receipts,
        }
    }
}

impl Contract {
    pub fn new(compiled_contract: CompiledContract, wallet: LocalWallet) -> Self {
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

    /// Creates an ABI call based on a function selector and
    /// the encoding of its call arguments, which is a slice of Tokens.
    /// It returns a prepared ContractCall that can further be used to
    /// make the actual transaction.
    /// This method is the underlying implementation of the functions
    /// generated from an ABI JSON spec, i.e, this is what's generated:
    /// quote! {
    ///     #doc
    ///     pub fn #name(&self #input) -> #result {
    ///         Contract::method_hash(#tokenized_signature, #arg)
    ///     }
    /// }
    /// For more details see `code_gen/functions_gen.rs`.
    /// Note that this needs a wallet because the contract instance needs a wallet for the calls
    pub fn method_hash<D: Tokenizable + Debug>(
        provider: &Provider,
        contract_id: Bech32ContractId,
        wallet: &LocalWallet,
        signature: Selector,
        output_param: Option<ParamType>,
        args: &[Token],
    ) -> Result<ContractCallHandler<D>, Error> {
        let encoded_args = ABIEncoder::encode(args).unwrap();
        let encoded_selector = signature;

        let tx_parameters = TxParameters::default();
        let call_parameters = CallParameters::default();

        let compute_custom_input_offset = Contract::should_compute_custom_input_offset(args);

        let contract_call = ContractCall {
            contract_id,
            encoded_selector,
            encoded_args,
            call_parameters,
            compute_custom_input_offset,
            variable_outputs: None,
            external_contracts: vec![],
            output_param,
        };

        Ok(ContractCallHandler {
            contract_call,
            tx_parameters,
            wallet: wallet.clone(),
            provider: provider.clone(),
            datatype: PhantomData,
        })
    }

    // If the data passed into the contract method is an integer or a
    // boolean, then the data itself should be passed. Otherwise, it
    // should simply pass a pointer to the data in memory. For more
    // information, see https://github.com/FuelLabs/sway/issues/1368.
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
                )
            })
    }

    /// Loads a compiled contract and deploys it to a running node
    pub async fn deploy(
        binary_filepath: &str,
        wallet: &LocalWallet,
        params: TxParameters,
        storage_configuration: StorageConfiguration,
    ) -> Result<Bech32ContractId, Error> {
        let mut compiled_contract =
            Contract::load_sway_contract(binary_filepath, &storage_configuration.storage_path)?;

        Self::merge_storage_vectors(&storage_configuration, &mut compiled_contract);

        Self::deploy_loaded(&(compiled_contract), wallet, params).await
    }

    /// Loads a compiled contract with salt and deploys it to a running node
    pub async fn deploy_with_parameters(
        binary_filepath: &str,
        wallet: &LocalWallet,
        params: TxParameters,
        storage_configuration: StorageConfiguration,
        salt: Salt,
    ) -> Result<Bech32ContractId, Error> {
        let mut compiled_contract = Contract::load_sway_contract_with_parameters(
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
        wallet: &LocalWallet,
        params: TxParameters,
    ) -> Result<Bech32ContractId, Error> {
        let (mut tx, contract_id) =
            Self::contract_deployment_transaction(compiled_contract, wallet, params).await?;

        let provider = wallet.get_provider()?;

        let chain_info = provider.chain_info().await?;

        wallet.sign_transaction(&mut tx).await?;
        tx.validate_without_signature(
            chain_info.latest_block.height.0,
            &chain_info.consensus_parameters.into(),
        )?;

        provider.send_transaction(&tx).await?;

        Ok(contract_id)
    }

    pub fn load_sway_contract(
        binary_filepath: &str,
        storage_path: &Option<String>,
    ) -> Result<CompiledContract, Error> {
        Self::load_sway_contract_with_parameters(
            binary_filepath,
            storage_path,
            Salt::from([0u8; 32]),
        )
    }

    pub fn load_sway_contract_with_parameters(
        binary_filepath: &str,
        storage_path: &Option<String>,
        salt: Salt,
    ) -> Result<CompiledContract, Error> {
        let extension = Path::new(binary_filepath).extension().unwrap();
        if extension != "bin" {
            return Err(Error::InvalidData(extension.to_str().unwrap().to_owned()));
        }
        let bin = std::fs::read(binary_filepath)?;

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
        wallet: &LocalWallet,
        params: TxParameters,
    ) -> Result<(Transaction, Bech32ContractId), Error> {
        let bytecode_witness_index = 0;
        let storage_slots: Vec<StorageSlot> = compiled_contract.storage_slots.clone();
        let witnesses = vec![compiled_contract.raw.clone().into()];

        let static_contracts = vec![];

        let (contract_id, state_root) = Self::compute_contract_id_and_state_root(compiled_contract);

        let outputs: Vec<Output> = vec![
            Output::contract_created(contract_id, state_root),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            // For now we use the BASE_ASSET_ID constant
            Output::change(wallet.address().into(), 0, BASE_ASSET_ID),
        ];

        // The first witness is the bytecode we're deploying.
        // So, the signature will be appended at position 1 of
        // the witness list.
        let coin_witness_index = 1;

        let inputs = wallet
            .get_asset_inputs_for_amount(
                AssetId::default(),
                DEFAULT_SPENDABLE_COIN_AMOUNT,
                coin_witness_index,
            )
            .await?;

        let tx = Transaction::create(
            params.gas_price,
            params.gas_limit,
            params.byte_price,
            params.maturity,
            bytecode_witness_index,
            compiled_contract.salt,
            static_contracts,
            storage_slots,
            inputs,
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
    pub encoded_args: Vec<u8>,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub compute_custom_input_offset: bool,
    pub variable_outputs: Option<Vec<Output>>,
    pub external_contracts: Vec<Bech32ContractId>,
    pub output_param: Option<ParamType>,
}

impl ContractCall {
    /// Based on the returned Contract's output_params and the receipts returned from a call,
    /// decode the values and return them.
    pub fn get_decoded_output(
        param_type: &ParamType,
        receipts: &mut Vec<Receipt>,
    ) -> Result<Token, Error> {
        // Multiple returns are handled as one `Tuple` (which has its own `ParamType`)

        let (encoded_value, index) = match param_type.get_return_location() {
            ReturnLocation::ReturnData => {
                match receipts.iter().find(|&receipt| receipt.data().is_some()) {
                    Some(r) => {
                        let index = receipts.iter().position(|elt| elt == r).unwrap();
                        (r.data().unwrap().to_vec(), Some(index))
                    }
                    None => (vec![], None),
                }
            }
            ReturnLocation::Return => {
                match receipts.iter().find(|&receipt| receipt.val().is_some()) {
                    Some(r) => {
                        let index = receipts.iter().position(|elt| elt == r).unwrap();
                        (r.val().unwrap().to_be_bytes().to_vec(), Some(index))
                    }
                    None => (vec![], None),
                }
            }
        };
        if let Some(i) = index {
            receipts.remove(i);
        }

        let decoded_value = ABIDecoder::decode_single(param_type, &encoded_value)?;
        Ok(decoded_value)
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct ContractCallHandler<D> {
    pub contract_call: ContractCall,
    pub tx_parameters: TxParameters,
    pub wallet: LocalWallet,
    pub provider: Provider,
    pub datatype: PhantomData<D>,
}

impl<D> ContractCallHandler<D>
where
    D: Tokenizable + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create Input::Contract/Output::Contract
    /// pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).set_contracts(&[another_contract_id]).call()`.
    pub fn set_contracts(mut self, contract_ids: &[Bech32ContractId]) -> Self {
        self.contract_call.external_contracts = contract_ids.to_vec();
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// let params = TxParameters { gas_price: 100, gas_limit: 1000000, byte_price: 100 };
    /// `my_contract_instance.my_method(...).tx_params(params).call()`.
    pub fn tx_params(mut self, params: TxParameters) -> Self {
        self.tx_parameters = params;
        self
    }

    /// Sets the call parameters for a given contract call.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// let params = CallParameters { amount: 1, asset_id: BASE_ASSET_ID };
    /// `my_contract_instance.my_method(...).call_params(params).call()`.
    pub fn call_params(mut self, params: CallParameters) -> Self {
        self.contract_call.call_parameters = params;
        self
    }

    /// Appends `num` `Output::Variable`s to the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).add_variable_outputs(num).call()`.
    pub fn append_variable_outputs(mut self, num: u64) -> Self {
        let new_outputs: Vec<Output> = (0..num)
            .map(|_| Output::Variable {
                amount: 0,
                to: Address::zeroed(),
                asset_id: AssetId::default(),
            })
            .collect();

        match self.contract_call.variable_outputs {
            Some(ref mut outputs) => outputs.extend(new_outputs),
            None => self.contract_call.variable_outputs = Some(new_outputs),
        }

        self
    }

    /// Call a contract's method on the node. If `simulate==true`, then the call is done in a
    /// read-only manner, using a `dry-run`. Return a Result<CallResponse, Error>. The CallResponse
    /// struct contains the method's value in its `value` field as an actual typed value `D` (if
    /// your method returns `bool`, it will be a bool, works also for structs thanks to the
    /// `abigen!()`). The other field of CallResponse, `receipts`, contains the receipts of the
    /// transaction.
    #[tracing::instrument]
    async fn call_or_simulate(self, simulate: bool) -> Result<CallResponse<D>, Error> {
        let script = self.get_script().await;

        let receipts = if simulate {
            script.simulate(&self.provider).await?
        } else {
            script.call(&self.provider).await?
        };
        tracing::debug!(target: "receipts", "{:?}", receipts);

        self.get_response(receipts)
    }

    /// Returns the script that executes the contract call
    pub async fn get_script(&self) -> Script {
        Script::from_contract_calls(vec![&self.contract_call], &self.tx_parameters, &self.wallet)
            .await
    }

    /// Call a contract's method on the node, in a state-modifying manner.
    pub async fn call(self) -> Result<CallResponse<D>, Error> {
        Self::call_or_simulate(self, false).await
    }

    /// Call a contract's method on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the `call` method because the API is more user-friendly this way.
    pub async fn simulate(self) -> Result<CallResponse<D>, Error> {
        Self::call_or_simulate(self, true).await
    }

    /// Create a CallResponse from call receipts
    pub fn get_response(&self, mut receipts: Vec<Receipt>) -> Result<CallResponse<D>, Error> {
        match self.contract_call.output_param.as_ref() {
            None => Ok(CallResponse::new(D::from_token(Token::Unit)?, receipts)),
            Some(param_type) => {
                let token = ContractCall::get_decoded_output(param_type, &mut receipts)?;
                Ok(CallResponse::new(D::from_token(token)?, receipts))
            }
        }
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles bundling multiple calls into a single transaction
pub struct MultiContractCallHandler {
    pub contract_calls: Option<Vec<ContractCall>>,
    pub tx_parameters: TxParameters,
    pub wallet: LocalWallet,
}

impl MultiContractCallHandler {
    pub fn new(wallet: LocalWallet) -> Self {
        Self {
            contract_calls: None,
            tx_parameters: TxParameters::default(),
            wallet,
        }
    }

    /// Adds a contract call to be bundled in the transaction
    /// Note that this is a builder method
    pub fn add_call<D: Tokenizable>(&mut self, call_handler: ContractCallHandler<D>) -> &mut Self {
        match self.contract_calls.as_mut() {
            Some(c) => c.push(call_handler.contract_call),
            None => self.contract_calls = Some(vec![call_handler.contract_call]),
        }
        self
    }

    /// Sets the transaction parameters for a given transaction.
    /// Note that this is a builder method
    pub fn tx_params(&mut self, params: TxParameters) -> &mut Self {
        self.tx_parameters = params;
        self
    }

    /// Returns the script that executes the contract calls
    pub async fn get_script(&self) -> Script {
        Script::from_contract_calls(
            self.contract_calls
                .as_ref()
                .expect("No calls added. Have you used '.add_calls()'?")
                .iter()
                .collect(),
            &self.tx_parameters,
            &self.wallet,
        )
        .await
    }

    /// Call contract methods on the node, in a state-modifying manner.
    pub async fn call<D: Tokenizable + Debug>(&self) -> Result<CallResponse<D>, Error> {
        Self::call_or_simulate(self, false).await
    }

    /// Call contract methods on the node, in a simulated manner, meaning the state of the
    /// blockchain is *not* modified but simulated.
    /// It is the same as the `call` method because the API is more user-friendly this way.
    pub async fn simulate<D: Tokenizable + Debug>(&self) -> Result<CallResponse<D>, Error> {
        Self::call_or_simulate(self, true).await
    }

    #[tracing::instrument]
    async fn call_or_simulate<D: Tokenizable + Debug>(
        &self,
        simulate: bool,
    ) -> Result<CallResponse<D>, Error> {
        let script = self.get_script().await;

        let provider = self.wallet.get_provider()?;

        let receipts = if simulate {
            script.simulate(provider).await.unwrap()
        } else {
            script.call(provider).await.unwrap()
        };
        tracing::debug!(target: "receipts", "{:?}", receipts);

        self.get_response(receipts)
    }

    /// Create a MultiCallResponse from call receipts
    pub fn get_response<D: Tokenizable + Debug>(
        &self,
        mut receipts: Vec<Receipt>,
    ) -> Result<CallResponse<D>, Error> {
        let mut final_tokens = vec![];

        for call in self.contract_calls.as_ref().unwrap().iter() {
            // We only aggregate the tokens if the contract call has an output parameter
            if let Some(param_type) = call.output_param.as_ref() {
                let decoded = ContractCall::get_decoded_output(param_type, &mut receipts)?;

                final_tokens.push(decoded.clone());
            }
        }

        let tokens_as_tuple = Token::Tuple(final_tokens);
        let response = CallResponse::<D>::new(D::from_token(tokens_as_tuple)?, receipts);

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use fuels_test_helpers::launch_provider_and_get_wallet;

    use super::*;

    #[tokio::test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidData(\"json\")")]
    async fn deploy_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy(
            "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidData(\"json\")")]
    async fn deploy_with_salt_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy_with_parameters(
            "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
            Salt::default(),
        )
        .await
        .unwrap();
    }
}
