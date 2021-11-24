use crate::abi_encoder::ABIEncoder;
use crate::errors::Error;
use crate::script::Script;
use crate::tokens::{Detokenize, Token};
use crate::types::Selector;
use core_types::Function;
use forc::test::{forc_build, BuildCommand};
use forc::util::helpers::{find_manifest_dir, read_manifest};
use fuel_asm::Opcode;
use fuel_client::client::FuelClient;
use fuel_core::service::{Config, FuelService};
use fuel_tx::{ContractId, Input, Output, Receipt, Transaction};
use fuel_types::{Address, Bytes32, Immediate12, Salt, Word};
use fuel_vm::consts::{REG_ZERO, VM_TX_MEMORY};
use fuel_vm::prelude::Contract as FuelContract;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::mem;
use std::path::PathBuf;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, Default)]
pub struct CompiledContract {
    pub raw: Vec<u8>,
    pub salt: Salt,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

/// Contract is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
pub struct Contract {
    pub compiled_contract: CompiledContract,
}

impl Contract {
    pub fn new(compiled_contract: CompiledContract) -> Self {
        Self { compiled_contract }
    }

    pub fn compute_contract_id(compiled_contract: &CompiledContract) -> ContractId {
        let fuel_contract = FuelContract::from(compiled_contract.raw.clone());
        let root = fuel_contract.root();
        fuel_contract.id(&compiled_contract.salt, &root)
    }

    /// Calls an already-deployed contract code.
    /// Note that this is a "generic" call to a contract
    /// and it doesn't, yet, call a specific ABI function in that contract.
    pub async fn call(
        contract_id: ContractId,
        fuel_client: &FuelClient,
        utxo_id: Bytes32,
        balance_root: Bytes32,
        state_root: Bytes32,
        input_index: u8,
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
    ) -> Result<Vec<Receipt>, String> {
        // Setup the script that will call a contract.
        // Register `0x10` will hold the beginning of the `script_data`
        // which will be computed below.
        let mut script_ops = vec![
            Opcode::ADDI(0x10, REG_ZERO, 0x00),
            Opcode::ADDI(0x11, 0x10, ContractId::LEN as Immediate12),
            Opcode::CALL(0x10, REG_ZERO, 0x10, 0x10),
            Opcode::RET(0x30),
        ];

        // @todo continue from here.
        // Try deploying a contract with proper functions
        // Then, try crafting `script_data` to contain
        // the function selector and the arguments.
        // Do it manually at first to validate that it works
        // Then try and generalize it.

        let script = script_ops.iter().copied().collect();

        // To call a contract, `script_data` should be
        // the ID of the contract we're trying to call
        // followed by 2 words.
        // Note that there are no arguments here yet. If we had,
        // we'd be extending `script_data` with it at the end.
        let mut script_data = contract_id.to_vec();
        script_data.extend(&[0u8; WORD_SIZE * 2]);

        // Inputs/outputs
        let input = Input::contract(utxo_id, balance_root, state_root, contract_id);
        let output = Output::contract(input_index, balance_root, state_root);

        let mut tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            script,
            script_data,
            vec![input],
            vec![output],
            vec![],
        );

        // Now that we know the length of the transaction `tx`
        // we can compute the script_data offset and update the script
        // `script_ops` with the proper Opcode.
        let script_data_offset = VM_TX_MEMORY + tx.script_data_offset().unwrap();
        script_ops[0] = Opcode::ADDI(0x10, REG_ZERO, script_data_offset as Immediate12);

        let script_mem: Vec<u8> = script_ops.iter().copied().collect();

        // Update the `script` property of the transaction.
        match &mut tx {
            Transaction::Script { script, .. } => {
                script.as_mut_slice().copy_from_slice(script_mem.as_slice())
            }
            _ => unreachable!(),
        }

        let script = Script::new(tx);

        Ok(script.call(&fuel_client).await.unwrap())
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
        compiled_contract: &CompiledContract,
        signature: Selector,
        args: &[Token],
    ) -> Result<ContractCall<D>, Error> {
        let mut encoder = ABIEncoder::new();

        let encoded_params = hex::encode(encoder.encode(args).unwrap());
        let encoded_selector = hex::encode(signature);

        // @todo soon, this method will make use of `self::call()`
        // to craft a call to a contract's function.
        // Right now this is an "empty" contract call.
        let tx = Transaction::Script {
            gas_price: 0,
            gas_limit: 1_000_000,
            maturity: 0,
            receipts_root: Default::default(),
            script: vec![],
            script_data: vec![],
            inputs: vec![Input::Coin {
                utxo_id: Bytes32::new([0u8; 32]),
                owner: Address::new([0u8; 32]),
                amount: 1,
                color: Default::default(),
                witness_index: 0,
                maturity: 0,
                predicate: vec![],
                predicate_data: vec![],
            }],
            outputs: vec![],
            witnesses: vec![vec![].into()],
            metadata: None,
        };

        Ok(ContractCall {
            compiled_contract: compiled_contract.clone(),
            encoded_params,
            encoded_selector,
            fuel_client: fuel_client.clone(), // cheap clone behind the Arc
            tx,
            function: None,
            datatype: PhantomData,
        })
    }

    /// Launches a local `fuel-core` network and deploys a contract to it.
    /// If you want to deploy a contract against another network of
    /// your choosing, use the `deploy` function instead.
    pub async fn launch_and_deploy(
        compiled_contract: &CompiledContract,
    ) -> Result<(FuelClient, ContractId), Error> {
        let srv = FuelService::new_node(Config::local_node()).await.unwrap();

        let fuel_client = FuelClient::from(srv.bound_address);

        let contract_id = Self::deploy(compiled_contract, &fuel_client).await?;

        Ok((fuel_client, contract_id))
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

    /// Compiles a Sway contract
    pub fn compile_sway_contract(
        project_path: &str,
        salt: Salt,
    ) -> Result<CompiledContract, Error> {
        let build_command = BuildCommand {
            path: Some(project_path.into()),
            print_finalized_asm: false,
            print_intermediate_asm: false,
            binary_outfile: None,
            offline_mode: false,
            silent_mode: true,
        };

        let raw =
            forc_build::build(build_command).map_err(|message| Error::CompilationError(message))?;

        let manifest_dir = find_manifest_dir(&PathBuf::from(project_path)).unwrap();
        let manifest = read_manifest(&manifest_dir).map_err(|e| {
            Error::CompilationError(format!("Failed to find manifest for contract: {}", e))
        })?;

        let (inputs, outputs) = manifest.get_tx_inputs_and_outputs().map_err(|e| {
            Error::CompilationError(format!(
                "Failed to find contract's inputs and outputs: {}",
                e
            ))
        })?;

        Ok(CompiledContract {
            salt,
            raw,
            inputs,
            outputs,
        })
    }

    /// Crafts a transaction used to deploy a contract
    pub fn contract_deployment_transaction(
        compiled_contract: &CompiledContract,
    ) -> (Transaction, ContractId) {
        // @todo get these configurations from
        // params of this function.
        let gas_price = 0;
        let gas_limit = 1000000;
        let maturity = 0;
        let bytecode_witness_index = 0;
        let witnesses = vec![compiled_contract.raw.clone().into()];

        let static_contracts = vec![];

        let contract_id = Self::compute_contract_id(compiled_contract);

        let output = Output::contract_created(contract_id);

        let tx = Transaction::create(
            gas_price,
            gas_limit,
            maturity,
            bytecode_witness_index,
            compiled_contract.salt,
            static_contracts,
            compiled_contract.inputs.clone(),
            vec![output],
            witnesses,
        );

        (tx, contract_id)
    }
}

/// Parameters for sending a transaction
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct TransactionRequest {
    /// The compiled code of a contract OR the first 4 bytes of the hash of the
    /// invoked method signature and encoded parameters. For details see Ethereum Contract ABI
    pub data: Option<Vec<u8>>,
    // More later
}

#[derive(Debug)]
#[must_use = "contract calls do nothing unless you `call` them"]
/// Helper for managing a transaction before submitting it to a node
pub struct ContractCall<D> {
    /// The raw transaction object
    pub tx: Transaction,

    pub fuel_client: FuelClient,

    pub compiled_contract: CompiledContract,
    /// The ABI of the function being called
    pub function: Option<Function>, // Temporarily an option

    pub datatype: PhantomData<D>,

    pub encoded_params: String,
    pub encoded_selector: String,
}

impl<D> ContractCall<D>
where
    D: Detokenize,
{
    pub async fn call(self) -> Result<Vec<Receipt>, Error> {
        let script = Script::new(self.tx);

        Ok(script.call(&self.fuel_client).await.unwrap())
    }
}
