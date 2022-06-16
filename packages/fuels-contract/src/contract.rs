use crate::{abi_decoder::ABIDecoder, abi_encoder::ABIEncoder, script::Script};
use anyhow::Result;
use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Contract as FuelContract, Output, Receipt, StorageSlot, Transaction},
    fuel_types::{Address, AssetId, ContractId, Salt},
};
use fuels_core::{
    constants::{BASE_ASSET_ID, DEFAULT_SPENDABLE_COIN_AMOUNT},
    errors::Error,
    parameters::{CallParameters, TxParameters},
    Detokenize, ParamType, ReturnLocation, Selector, Token,
};
use fuels_signers::{provider::Provider, LocalWallet, Signer};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    pub raw: Vec<u8>,
    pub salt: Salt,
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
pub struct CallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
    pub logs: Vec<String>,
}

impl<D> CallResponse<D> {
    pub fn new(value: D, receipts: Vec<Receipt>) -> Self {
        // Get all the logs from LogData receipts and put them in the `logs` property
        let logs_vec = receipts
            .iter()
            .filter(|r| matches!(r, Receipt::LogData { .. }))
            .map(|r| hex::encode(r.data().unwrap()))
            .collect::<Vec<String>>();
        Self {
            value,
            receipts,
            logs: logs_vec,
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

    pub fn compute_contract_id(compiled_contract: &CompiledContract) -> ContractId {
        let fuel_contract = FuelContract::from(compiled_contract.raw.clone());
        let root = fuel_contract.root();
        fuel_contract.id(
            &compiled_contract.salt,
            &root,
            &FuelContract::default_state_root(),
        )
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
    pub fn method_hash<D: Detokenize + Debug>(
        provider: &Provider,
        contract_id: ContractId,
        wallet: &LocalWallet,
        signature: Selector,
        output_params: &[ParamType],
        args: &[Token],
    ) -> Result<ContractCallHandler<D>, Error> {
        let mut encoder = ABIEncoder::new();

        let encoded_args = encoder.encode(args).unwrap();
        let encoded_selector = signature;

        let tx_parameters = TxParameters::default();
        let call_parameters = CallParameters::default();

        let compute_custom_input_offset = Contract::should_compute_custom_input_offset(args);

        let maturity = 0;

        let contract_call = ContractCall {
            contract_id,
            encoded_selector,
            encoded_args,
            call_parameters,
            maturity,
            compute_custom_input_offset,
            variable_outputs: None,
            external_contracts: None,
            output_params: output_params.to_vec(),
        };

        Ok(ContractCallHandler {
            contract_call,
            tx_parameters,
            wallet: wallet.clone(),
            fuel_client: provider.client.clone(),
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
    ) -> Result<ContractId, Error> {
        let compiled_contract = Contract::load_sway_contract(binary_filepath)?;
        Self::deploy_loaded(&(compiled_contract), wallet, params).await
    }

    /// Loads a compiled contract with salt and deploys it to a running node
    pub async fn deploy_with_salt(
        binary_filepath: &str,
        wallet: &LocalWallet,
        params: TxParameters,
        salt: Salt,
    ) -> Result<ContractId, Error> {
        let compiled_contract = Contract::load_sway_contract_with_salt(binary_filepath, salt)?;
        Self::deploy_loaded(&(compiled_contract), wallet, params).await
    }

    /// Deploys a compiled contract to a running node
    /// To deploy a contract, you need a wallet with enough assets to pay for deployment. This
    /// wallet will also receive the change.
    pub async fn deploy_loaded(
        compiled_contract: &CompiledContract,
        wallet: &LocalWallet,
        params: TxParameters,
    ) -> Result<ContractId, Error> {
        let (mut tx, contract_id) =
            Self::contract_deployment_transaction(compiled_contract, wallet, params).await?;
        wallet.sign_transaction(&mut tx).await?;

        match wallet.get_provider().unwrap().client.submit(&tx).await {
            Ok(_) => Ok(contract_id),
            Err(e) => Err(Error::TransactionError(e.to_string())),
        }
    }

    pub fn load_sway_contract(binary_filepath: &str) -> Result<CompiledContract, Error> {
        Self::load_sway_contract_with_salt(binary_filepath, Salt::from([0u8; 32]))
    }

    pub fn load_sway_contract_with_salt(
        binary_filepath: &str,
        salt: Salt,
    ) -> Result<CompiledContract, Error> {
        let extension = Path::new(binary_filepath).extension().unwrap();
        if extension != "bin" {
            return Err(Error::InvalidData(extension.to_str().unwrap().to_owned()));
        }
        let bin = std::fs::read(binary_filepath)?;
        Ok(CompiledContract { raw: bin, salt })
    }

    /// Crafts a transaction used to deploy a contract
    pub async fn contract_deployment_transaction(
        compiled_contract: &CompiledContract,
        wallet: &LocalWallet,
        params: TxParameters,
    ) -> Result<(Transaction, ContractId), Error> {
        let maturity = 0;
        let bytecode_witness_index = 0;
        let storage_slots: Vec<StorageSlot> = vec![];
        let witnesses = vec![compiled_contract.raw.clone().into()];

        let static_contracts = vec![];

        let contract_id = Self::compute_contract_id(compiled_contract);

        let outputs: Vec<Output> = vec![
            Output::contract_created(contract_id, FuelContract::default_state_root()),
            // Note that the change will be computed by the node.
            // Here we only have to tell the node who will own the change and its asset ID.
            // For now we use the BASE_ASSET_ID constant
            Output::change(wallet.address(), 0, BASE_ASSET_ID),
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
            maturity,
            bytecode_witness_index,
            compiled_contract.salt,
            static_contracts,
            storage_slots,
            inputs,
            outputs,
            witnesses,
        );

        Ok((tx, contract_id))
    }
}

#[derive(Debug)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: ContractId,
    pub encoded_args: Vec<u8>,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub maturity: u64,
    pub compute_custom_input_offset: bool,
    pub variable_outputs: Option<Vec<Output>>,
    pub external_contracts: Option<Vec<ContractId>>,
    pub output_params: Vec<ParamType>,
}

impl ContractCall {
    /// Based on the returned Contract's output_params and the receipts returned from a call,
    /// decode the values and return them.
    pub fn get_decoded_output(
        &self,
        mut receipts: Vec<Receipt>,
    ) -> Result<(Vec<Token>, Vec<Receipt>), Error> {
        // Multiple returns are handled as one `Tuple` (which has its own `ParamType`), so getting
        // more than one output param is an error.
        if self.output_params.len() != 1 {
            return Err(Error::InvalidType(format!(
                "Received too many output params (expected 1 got {})",
                self.output_params.len()
            )));
        }
        let output_param = self.output_params[0].clone();

        let (encoded_value, index) = match output_param.get_return_location() {
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
        let decoded_value = ABIDecoder::decode(&self.output_params, &encoded_value)?;
        Ok((decoded_value, receipts))
    }
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper that handles submitting a call to a client and formatting the response
pub struct ContractCallHandler<D> {
    pub contract_call: ContractCall,
    pub tx_parameters: TxParameters,
    pub wallet: LocalWallet,
    pub fuel_client: FuelClient,
    pub datatype: PhantomData<D>,
}

impl<D> ContractCallHandler<D>
where
    D: Detokenize + Debug,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create Input::Contract/Output::Contract
    /// pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).set_contracts(&[another_contract_id]).call()`.
    pub fn set_contracts(mut self, contract_ids: &[ContractId]) -> Self {
        self.contract_call.external_contracts = Some(contract_ids.to_vec());
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
            script.simulate(&self.fuel_client).await?
        } else {
            script.call(&self.fuel_client).await?
        };
        tracing::debug!(target: "receipts", "{:?}", receipts);

        self.get_response(receipts)
    }

    pub async fn get_script(&self) -> Script {
        Script::from_contract_call(&self.contract_call, &self.tx_parameters, &self.wallet).await
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
    pub fn get_response(&self, receipts: Vec<Receipt>) -> Result<CallResponse<D>, Error> {
        // If it's an ABI method without a return value, exit early.
        if self.contract_call.output_params.is_empty() {
            return Ok(CallResponse::new(D::from_tokens(vec![])?, receipts));
        }

        let (decoded_value, receipts) = self.contract_call.get_decoded_output(receipts)?;
        Ok(CallResponse::new(D::from_tokens(decoded_value)?, receipts))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use fuels_test_helpers::launch_provider_and_get_single_wallet;

    #[tokio::test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidData(\"json\")")]
    async fn deploy_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_single_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy(
            "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidData(\"json\")")]
    async fn deploy_with_salt_panics_on_non_binary_file() {
        let wallet = launch_provider_and_get_single_wallet().await;

        // Should panic as we are passing in a JSON instead of BIN
        Contract::deploy_with_salt(
            "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
            &wallet,
            TxParameters::default(),
            Salt::default(),
        )
        .await
        .unwrap();
    }
}
