use std::{collections::HashSet, fmt::Debug, fs, marker::PhantomData, path::Path, str::FromStr};

use fuel_tx::{
    Bytes32, Checkable, Contract as FuelContract, ContractId, Create, Output, Salt, StorageSlot,
    Transaction,
};
use fuels_core::{
    abi_encoder::ABIEncoder,
    parameters::{CallParameters, StorageConfiguration, TxParameters},
    traits::{Parameterize, Tokenizable},
};
use fuels_signers::{provider::Provider, Signer, WalletUnlocked};
use fuels_types::{
    bech32::Bech32ContractId,
    core::{Selector, Token},
    errors::Error,
};

use crate::contract_call::{ContractCall, ContractCallHandler};
use crate::logs::LogDecoder;

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
            variable_outputs: vec![],
            message_outputs: vec![],
            external_contracts: vec![],
            output_param: D::param_type(),
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
            "tests/types/contract_output_test/out/debug/contract_output_test-abi.json",
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
