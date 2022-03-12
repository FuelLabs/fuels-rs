use crate::abi_decoder::ABIDecoder;
use crate::abi_encoder::ABIEncoder;
use crate::errors::Error;
use crate::script::Script;
use anyhow::Result;
use fuel_asm::Opcode;
use fuel_gql_client::client::FuelClient;
use fuel_tx::{
    Address, AssetId, ContractId, Input, Output, Receipt, StorageSlot, Transaction, UtxoId, Witness,
};
use fuel_types::{Bytes32, Immediate12, Salt, Word};
use fuel_vm::consts::{REG_CGAS, REG_RET, REG_ZERO, VM_TX_MEMORY};
use fuel_vm::prelude::Contract as FuelContract;
use fuels_core::ParamType;
use fuels_core::{Detokenize, Selector, Token, WORD_SIZE};
use rand::{Rng, RngCore};
use std::marker::PhantomData;

#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    pub raw: Vec<u8>,
    pub salt: Salt,
}

/// Contract is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
pub struct Contract {
    pub compiled_contract: CompiledContract,
}

/// CallResponse is a struct that is returned by a call to the contract. Its value field
/// holds the decoded typed value returned by the contract's method. The other field
/// holds all the receipts returned by the call.
#[derive(Debug)]
pub struct CallResponse<D> {
    pub value: D,
    pub receipts: Vec<Receipt>,
}

impl Contract {
    pub fn new(compiled_contract: CompiledContract) -> Self {
        Self { compiled_contract }
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

    /// Calls an already-deployed contract code.
    /// Note that this is a "generic" call to a contract
    /// and it doesn't, yet, call a specific ABI function in that contract.
    #[allow(clippy::too_many_arguments)] // We need that many arguments for now
    pub async fn call(
        contract_id: ContractId,
        encoded_selector: Option<Selector>,
        encoded_args: Option<Vec<u8>>,
        fuel_client: &FuelClient,
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        custom_inputs: bool,
        external_contracts: Option<Vec<ContractId>>,
    ) -> Result<Vec<Receipt>, Error> {
        // Based on the defined script length, we set the appropriate data offset.
        let script_len = 16;
        let script_data_offset = VM_TX_MEMORY + Transaction::script_offset() + script_len;
        let script_data_offset = script_data_offset as Immediate12;

        // Script to call the contract. The offset that points to the `script_data` is loaded at the
        // register `0x10`. Note that we're picking `0x10` simply because it could be any
        // non-reserved register. Then, we use the Opcode to call a contract: `CALL` pointing at the
        // register that we loaded the `script_data` at.

        // The `iter().collect()` does the Opcode->u8 conversion
        #[allow(clippy::iter_cloned_collect)]
        let script = vec![
            Opcode::ADDI(0x10, REG_ZERO, script_data_offset),
            Opcode::CALL(0x10, REG_ZERO, 0x10, REG_CGAS),
            Opcode::RET(REG_RET),
            Opcode::NOOP,
        ]
        .iter()
        .copied()
        .collect::<Vec<u8>>();

        assert!(script.len() == script_len, "Script length *must* be 16");

        // `script_data` consists of:
        // 1. Contract ID (ContractID::LEN);
        // 2. Function selector (1 * WORD_SIZE);
        // 3. Calldata offset, if it has structs as input,
        // computed as `script_data_offset` + ContractId::LEN
        //                                  + 2 * WORD_SIZE;
        // 4. Encoded arguments.
        let mut script_data: Vec<u8> = vec![];

        // Insert contract_id
        script_data.extend(contract_id.as_ref());

        // Insert encoded function selector, if any
        if let Some(e) = encoded_selector {
            script_data.extend(e)
        }

        // If the method call takes custom inputs, such as structs or enums, we need to calculate
        // the `call_data_offset`, which points to where the data for the custom types start in the
        // transaction. If it doesn't take any custom inputs, this isn't necessary.
        if custom_inputs {
            // Offset of the script data relative to the call data
            let call_data_offset = script_data_offset as usize + ContractId::LEN + 2 * WORD_SIZE;
            let call_data_offset = call_data_offset as Word;

            script_data.extend(&call_data_offset.to_be_bytes());
        }

        // Insert encoded arguments, if any
        if let Some(e) = encoded_args {
            script_data.extend(e)
        }

        let mut inputs: Vec<Input> = vec![];
        let mut outputs: Vec<Output> = vec![];

        let self_contract_input = Input::contract(
            UtxoId::new(Bytes32::zeroed(), 0),
            Bytes32::zeroed(),
            Bytes32::zeroed(),
            contract_id,
        );
        inputs.push(self_contract_input);

        let self_contract_output = Output::contract(0, Bytes32::zeroed(), Bytes32::zeroed());
        outputs.push(self_contract_output);

        // Add external contract IDs to Input/Output pair, if applicable.
        if let Some(external_contract_ids) = external_contracts {
            for (idx, external_contract_id) in external_contract_ids.iter().enumerate() {
                let external_contract_input = Input::contract(
                    UtxoId::new(Bytes32::zeroed(), idx as u8 + 1),
                    Bytes32::zeroed(),
                    Bytes32::zeroed(),
                    *external_contract_id,
                );

                inputs.push(external_contract_input);

                let external_contract_output =
                    Output::contract(idx as u8 + 1, Bytes32::zeroed(), Bytes32::zeroed());

                outputs.push(external_contract_output);
            }
        }

        // Important: this is a temporary workaround, for full context:
        // https://github.com/FuelLabs/fuels-rs/issues/90
        // Here we generate a random coin input to prevent transaction id
        // collision in method calls that could be view-only.
        let mut rng = rand::thread_rng();

        let random_coin = Input::coin(
            UtxoId::new(Bytes32::from(rng.gen::<[u8; 32]>()), 0),
            Address::from(rng.gen::<[u8; 32]>()),
            rng.next_u64(),
            AssetId::from([0u8; 32]),
            0,
            0,
            vec![],
            vec![],
        );

        inputs.push(random_coin);

        let tx = Transaction::script(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            script,
            script_data,
            inputs,
            outputs,
            vec![Witness::from(vec![0u8, 0u8])],
        );

        let script = Script::new(tx);

        script.call(fuel_client).await
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
    pub fn method_hash<D: Detokenize>(
        fuel_client: &FuelClient,
        contract_id: ContractId,
        signature: Selector,
        output_params: &[ParamType],
        args: &[Token],
    ) -> Result<ContractCall<D>, Error> {
        let mut encoder = ABIEncoder::new();

        let encoded_args = encoder.encode(args).unwrap();
        let encoded_selector = signature;

        let gas_price = 0;
        let byte_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;

        let custom_inputs = args.iter().any(|t| matches!(t, Token::Struct(_)));

        Ok(ContractCall {
            contract_id,
            encoded_args,
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            encoded_selector,
            fuel_client: fuel_client.clone(),
            datatype: PhantomData,
            output_params: output_params.to_vec(),
            custom_inputs,
            external_contracts: None,
        })
    }

    /// Deploys a compiled contract to a running node
    pub async fn deploy(
        compiled_contract: &CompiledContract,
        fuel_client: &FuelClient,
    ) -> Result<ContractId, Error> {
        let (tx, contract_id) = Self::contract_deployment_transaction(compiled_contract);

        match fuel_client.submit(&tx).await {
            Ok(_) => Ok(contract_id),
            Err(e) => Err(Error::TransactionError(e.to_string())),
        }
    }

    pub fn load_sway_contract(binary_filepath: &str, salt: Salt) -> Result<CompiledContract> {
        let bin = std::fs::read(binary_filepath)?;
        Ok(CompiledContract { raw: bin, salt })
    }

    /// Crafts a transaction used to deploy a contract
    pub fn contract_deployment_transaction(
        compiled_contract: &CompiledContract,
    ) -> (Transaction, ContractId) {
        // @todo get these configurations from
        // params of this function.
        let gas_price = 0;
        let byte_price = 0;
        let gas_limit = 1000000;
        let maturity = 0;
        let bytecode_witness_index = 0;
        let storage_slots: Vec<StorageSlot> = vec![];
        let witnesses = vec![compiled_contract.raw.clone().into()];

        let static_contracts = vec![];

        let contract_id = Self::compute_contract_id(compiled_contract);

        let output = Output::contract_created(contract_id, FuelContract::default_state_root());

        let tx = Transaction::create(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            bytecode_witness_index,
            compiled_contract.salt,
            static_contracts,
            storage_slots,
            vec![],
            vec![output],
            witnesses,
        );

        (tx, contract_id)
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
    pub gas_price: u64,
    pub gas_limit: u64,
    pub maturity: u64,
    pub byte_price: u64,
    pub datatype: PhantomData<D>,
    pub output_params: Vec<ParamType>,
    pub custom_inputs: bool,
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

    /// Call a contract's method. Return a Result<CallResponse, Error>.
    /// The CallResponse structs contains the method's value in its `value`
    /// field as an actual typed value `D` (if your method returns `bool`, it will
    /// be a bool, works also for structs thanks to the `abigen!()`).
    /// The other field of CallResponse, `receipts`, contains the receipts of the
    /// transaction
    pub async fn call(self) -> Result<CallResponse<D>, Error> {
        let mut receipts = Contract::call(
            self.contract_id,
            Some(self.encoded_selector),
            Some(self.encoded_args),
            &self.fuel_client,
            self.gas_price,
            self.gas_limit,
            self.byte_price,
            self.maturity,
            self.custom_inputs,
            self.external_contracts,
        )
        .await?;

        // If it's an ABI method without a return value, exit early.
        if self.output_params.is_empty() {
            return Ok(CallResponse {
                value: D::from_tokens(vec![])?,
                receipts,
            });
        }

        // Right now we only support methods with a single return type.
        // Soon we'll support tuple as a return type and we'll have to update the logic in here.
        let output_param = &self.output_params[0];

        // If the method's return type is bigger than a single `WORD`, the returned value
        // is stored in `ReturnData.data`, otherwise, it's stored in `Return.val`.
        // Here we're checking for that.
        let (encoded_value, index) = match output_param.bigger_than_word() {
            true => match receipts.iter().find(|&receipt| receipt.data().is_some()) {
                Some(r) => {
                    let index = receipts.iter().position(|elt| elt == r).unwrap();
                    (r.data().unwrap().to_vec(), Some(index))
                }
                None => (vec![], None),
            },
            false => match receipts.iter().find(|&receipt| receipt.val().is_some()) {
                Some(r) => {
                    let index = receipts.iter().position(|elt| elt == r).unwrap();
                    (r.val().unwrap().to_be_bytes().to_vec(), Some(index))
                }
                None => (vec![], None),
            },
        };

        if index.is_some() {
            receipts.remove(index.unwrap());
        }
        let mut decoder = ABIDecoder::new();
        let decoded_value = decoder.decode(&self.output_params, &encoded_value)?;
        Ok(CallResponse {
            value: D::from_tokens(decoded_value)?,
            receipts,
        })
    }
}
