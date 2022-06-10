use crate::{abi_decoder::ABIDecoder, abi_encoder::ABIEncoder, script::Script};
use anyhow::Result;
use fuel_gql_client::{
    client::FuelClient,
    fuel_tx::{Contract as FuelContract, Input, Output, Receipt, StorageSlot, Transaction, UtxoId},
    fuel_types::{Address, AssetId, Bytes32, ContractId, Salt, Word},
    fuel_vm::{
        consts::{REG_CGAS, REG_ONE},
        prelude::Opcode,
        script_with_data_offset,
    },
};
use fuels_core::{
    constants::{BASE_ASSET_ID, DEFAULT_SPENDABLE_COIN_AMOUNT, WORD_SIZE},
    errors::Error,
    parameters::{CallParameters, TxParameters},
    Detokenize, ParamType, ReturnLocation, Selector, Token,
};
use fuels_signers::{provider::Provider, LocalWallet, Signer};
use std::marker::PhantomData;

use tracing::debug;

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

    /// Given the necessary arguments, create a script that will be submitted to the node to call
    /// the contract. The script is the actual opcodes used to call the contract, and the script
    /// data is for instance the function selector. (script, script_data) is returned as a tuple
    /// of hex-encoded value vectors
    pub fn build_script(
        contract_id: &ContractId,
        encoded_selector: &Option<Selector>,
        encoded_args: &Option<Vec<u8>>,
        call_parameters: &CallParameters,
        compute_calldata_offset: bool,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        use fuel_gql_client::fuel_types;
        // Script to call the contract.
        // We use the Opcode to call a contract: `CALL` pointing at the
        // following registers;
        //
        // 0x10 Script data offset
        // 0x11 Gas price  TODO: https://github.com/FuelLabs/fuels-rs/issues/184
        // 0x12 Coin amount
        // 0x13 Asset ID
        //
        // Note that these are soft rules as we're picking this addresses simply because they
        // non-reserved register.
        let forward_data_offset = ContractId::LEN + WORD_SIZE;
        let (script, offset) = script_with_data_offset!(
            data_offset,
            vec![
                // Load call data to 0x10.
                Opcode::MOVI(0x10, data_offset + forward_data_offset as Immediate18),
                // Load gas forward to 0x11.
                // Load word into 0x12
                Opcode::MOVI(
                    0x12,
                    ((data_offset as usize) + ContractId::LEN) as Immediate18
                ),
                // Load the amount into 0x12
                Opcode::LW(0x12, 0x12, 0),
                // Load the asset id to use to 0x13.
                Opcode::MOVI(0x13, data_offset),
                // Call the transfer contract.
                Opcode::CALL(0x10, 0x12, 0x13, REG_CGAS),
                Opcode::RET(REG_ONE),
            ]
        );

        #[allow(clippy::iter_cloned_collect)]
        let script = script.iter().copied().collect::<Vec<u8>>();

        // `script_data` consists of:
        // 1. Asset ID to be forwarded
        // 2. Amount to be forwarded
        // 3. Contract ID (ContractID::LEN);
        // 4. Function selector (1 * WORD_SIZE);
        // 5. Calldata offset, if it has structs as input,
        // computed as `script_data_offset` + ContractId::LEN
        //                                  + 2 * WORD_SIZE;
        // 6. Encoded arguments.
        let mut script_data: Vec<u8> = vec![];

        // Insert asset_id to be forwarded
        script_data.extend(call_parameters.asset_id.to_vec());

        // Insert amount to be forwarded
        let amount = call_parameters.amount as Word;
        script_data.extend(amount.to_be_bytes());

        // Insert contract_id
        script_data.extend(contract_id.as_ref());

        // Insert encoded function selector, if any
        if let Some(e) = encoded_selector {
            script_data.extend(e)
        }

        // If the method call takes custom inputs or has more than
        // one argument, we need to calculate the `call_data_offset`,
        // which points to where the data for the custom types start in the
        // transaction. If it doesn't take any custom inputs, this isn't necessary.
        if compute_calldata_offset {
            // Offset of the script data relative to the call data
            let call_data_offset =
                ((offset as usize) + forward_data_offset) + ContractId::LEN + 2 * WORD_SIZE;
            let call_data_offset = call_data_offset as Word;

            script_data.extend(&call_data_offset.to_be_bytes());
        }

        // Insert encoded arguments, if any
        if let Some(e) = encoded_args {
            script_data.extend(e)
        }
        Ok((script, script_data))
    }

    /// Calls a contract method with the given ABI function.
    /// The wallet is here to pay for the transaction fees (even though they are 0 right now)
    #[tracing::instrument]
    #[allow(clippy::too_many_arguments)] // We need that many arguments for now
    async fn call(
        contract_id: ContractId,
        encoded_selector: Option<Selector>,
        encoded_args: Option<Vec<u8>>,
        fuel_client: &FuelClient,
        tx_parameters: TxParameters,
        call_parameters: CallParameters,
        variable_outputs: Option<Vec<Output>>,
        maturity: Word,
        compute_calldata_offset: bool,
        external_contracts: Option<Vec<ContractId>>,
        wallet: LocalWallet,
        simulate: bool,
    ) -> Result<Vec<Receipt>, Error> {
        let (script, script_data) = Self::build_script(
            &contract_id,
            &encoded_selector,
            &encoded_args,
            &call_parameters,
            compute_calldata_offset,
        )?;
        let mut inputs: Vec<Input> = vec![];
        let mut outputs: Vec<Output> = vec![];

        let self_contract_input = Input::contract(
            UtxoId::new(Bytes32::zeroed(), 0),
            Bytes32::zeroed(),
            Bytes32::zeroed(),
            contract_id,
        );
        inputs.push(self_contract_input);

        let mut spendables = wallet
            .get_spendable_coins(&AssetId::default(), DEFAULT_SPENDABLE_COIN_AMOUNT as u64)
            .await
            .unwrap();

        // add default asset change if any inputs are being spent
        if !spendables.is_empty() {
            let change_output = Output::change(wallet.address(), 0, AssetId::default());
            outputs.push(change_output);
        }

        if call_parameters.asset_id != AssetId::default() {
            let alt_spendables = wallet
                .get_spendable_coins(&call_parameters.asset_id, call_parameters.amount)
                .await
                .unwrap();

            // add alt change if inputs are being spent
            if !alt_spendables.is_empty() {
                let change_output = Output::change(wallet.address(), 0, call_parameters.asset_id);
                outputs.push(change_output);
            }

            // add alt coins to inputs
            spendables.extend(alt_spendables.into_iter());
        }

        for coin in spendables {
            let input_coin = Input::coin_signed(
                UtxoId::from(coin.utxo_id),
                coin.owner.into(),
                coin.amount.0,
                coin.asset_id.into(),
                0,
                0,
            );

            inputs.push(input_coin);
        }

        let n_inputs = inputs.len();

        let self_contract_output = Output::contract(0, Bytes32::zeroed(), Bytes32::zeroed());
        outputs.push(self_contract_output);

        // Add external contract IDs to Input/Output pair, if applicable.
        if let Some(external_contract_ids) = external_contracts {
            for (idx, external_contract_id) in external_contract_ids.iter().enumerate() {
                // We must associate the right external contract input to the corresponding external
                // output index (TXO). We add the `n_inputs` offset because we added some inputs
                // above.
                let output_index: u8 = (idx + n_inputs) as u8;
                let zeroes = Bytes32::zeroed();
                let external_contract_input = Input::contract(
                    UtxoId::new(Bytes32::zeroed(), output_index),
                    zeroes,
                    zeroes,
                    *external_contract_id,
                );

                inputs.push(external_contract_input);

                let external_contract_output = Output::contract(output_index, zeroes, zeroes);

                outputs.push(external_contract_output);
            }
        }

        // Add outputs to the transaction.
        if let Some(v) = variable_outputs {
            outputs.extend(v);
        };

        let mut tx = Transaction::script(
            tx_parameters.gas_price,
            tx_parameters.gas_limit,
            tx_parameters.byte_price,
            maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![],
        );
        wallet.sign_transaction(&mut tx).await?;

        let script = Script::new(tx);

        if simulate {
            let receipts = script.simulate(fuel_client).await;
            debug!(target: "receipts", "{:?}", receipts);
            return receipts;
        }
        let receipts = script.call(fuel_client).await;
        debug!(target: "receipts", "{:?}", receipts);
        receipts
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
    pub fn method_hash<D: Detokenize>(
        provider: &Provider,
        contract_id: ContractId,
        wallet: &LocalWallet,
        signature: Selector,
        output_params: &[ParamType],
        args: &[Token],
    ) -> Result<ContractCall<D>, Error> {
        let mut encoder = ABIEncoder::new();

        let encoded_args = encoder.encode(args).unwrap();
        let encoded_selector = signature;

        let tx_parameters = TxParameters::default();
        let call_parameters = CallParameters::default();

        let compute_calldata_offset = Contract::should_compute_call_data_offset(args);

        let maturity = 0;
        Ok(ContractCall {
            contract_id,
            encoded_args,
            tx_parameters,
            call_parameters,
            maturity,
            encoded_selector,
            fuel_client: provider.client.clone(),
            datatype: PhantomData,
            output_params: output_params.to_vec(),
            variable_outputs: None,
            compute_calldata_offset,
            external_contracts: None,
            wallet: wallet.clone(),
        })
    }

    // If the data passed into the contract method is an integer or a
    // boolean, then the data itself should be passed. Otherwise, it
    // should simply pass a pointer to the data in memory. For more
    // information, see https://github.com/FuelLabs/sway/issues/1368.
    fn should_compute_call_data_offset(args: &[Token]) -> bool {
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
        let compiled_contract = Contract::load_sway_contract(binary_filepath).unwrap();

        Self::deploy_loaded(&compiled_contract, wallet, params).await
    }

    /// Loads a compiled contract with salt and deploys it to a running node
    pub async fn deploy_with_salt(
        binary_filepath: &str,
        wallet: &LocalWallet,
        params: TxParameters,
        salt: Salt,
    ) -> Result<ContractId, Error> {
        let compiled_contract =
            Contract::load_sway_contract_with_salt(binary_filepath, salt).unwrap();

        Self::deploy_loaded(&compiled_contract, wallet, params).await
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

    pub fn load_sway_contract(binary_filepath: &str) -> Result<CompiledContract> {
        Self::load_sway_contract_with_salt(binary_filepath, Salt::from([0u8; 32]))
    }

    pub fn load_sway_contract_with_salt(
        binary_filepath: &str,
        salt: Salt,
    ) -> Result<CompiledContract> {
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
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper for managing a transaction before submitting it to a node
pub struct ContractCall<D> {
    pub fuel_client: FuelClient,
    pub encoded_args: Vec<u8>,
    pub encoded_selector: Selector,
    pub contract_id: ContractId,
    pub tx_parameters: TxParameters,
    pub call_parameters: CallParameters,
    pub maturity: u64,
    pub datatype: PhantomData<D>,
    pub output_params: Vec<ParamType>,
    pub compute_calldata_offset: bool,
    pub wallet: LocalWallet,
    pub variable_outputs: Option<Vec<Output>>,
    external_contracts: Option<Vec<ContractId>>,
}

impl<D> ContractCall<D>
where
    D: Detokenize,
{
    /// Sets external contracts as dependencies to this contract's call.
    /// Effectively, this will be used to create Input::Contract/Output::Contract
    /// pairs and set them into the transaction.
    /// Note that this is a builder method, i.e. use it as a chain:
    /// `my_contract_instance.my_method(...).set_contracts(&[another_contract_id]).call()`.
    pub fn set_contracts(mut self, contract_ids: &[ContractId]) -> Self {
        self.external_contracts = Some(contract_ids.to_vec());
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
        self.call_parameters = params;
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

        match self.variable_outputs {
            Some(ref mut outputs) => outputs.extend(new_outputs),
            None => self.variable_outputs = Some(new_outputs),
        }

        self
    }

    /// Call a contract's method on the node. If `simulate==true`, then the call is done in a
    /// read-only manner, using a `dry-run`. Return a Result<CallResponse, Error>. The CallResponse
    /// struct contains the method's value in its `value` field as an actual typed value `D` (if
    /// your method returns `bool`, it will be a bool, works also for structs thanks to the
    /// `abigen!()`). The other field of CallResponse, `receipts`, contains the receipts of the
    /// transaction.
    async fn call_or_simulate(self, simulate: bool) -> Result<CallResponse<D>, Error> {
        let receipts = Contract::call(
            self.contract_id,
            Some(self.encoded_selector),
            Some(self.encoded_args),
            &self.fuel_client,
            self.tx_parameters,
            self.call_parameters,
            self.variable_outputs,
            self.maturity,
            self.compute_calldata_offset,
            self.external_contracts,
            self.wallet,
            simulate,
        )
        .await?;

        // If it's an ABI method without a return value, exit early.
        if self.output_params.is_empty() {
            return Ok(CallResponse::new(D::from_tokens(vec![])?, receipts));
        }

        let (decoded_value, receipts) = Self::get_decoded_output(receipts, &self.output_params)?;
        Ok(CallResponse::new(D::from_tokens(decoded_value)?, receipts))
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

    /// Based on the returned Contract's output_params and the receipts returned from the call,
    /// decode the values and return them.
    pub fn get_decoded_output(
        mut receipts: Vec<Receipt>,
        output_params: &[ParamType],
    ) -> Result<(Vec<Token>, Vec<Receipt>), Error> {
        // Multiple returns are handled as one `Tuple` (which has its own `ParamType`), so getting
        // more than one output param is an error.
        if output_params.len() != 1 {
            return Err(Error::InvalidType(format!(
                "Received too many output params (expected 1 got {})",
                output_params.len()
            )));
        }
        let output_param = output_params[0].clone();

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
        let mut decoder = ABIDecoder::new();
        let decoded_value = decoder.decode(output_params, &encoded_value)?;
        Ok((decoded_value, receipts))
    }
}
