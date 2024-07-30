mod load;
mod storage;

use std::{
    collections::HashSet,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use fuel_asm::{op, Instruction, RegId};
use fuel_tx::{Bytes32, Contract as FuelContract, ContractId, Salt, StorageSlot};
use fuel_types::bytes::WORD_SIZE;
use fuels_accounts::{provider::Provider, Account};
use fuels_core::types::{
    bech32::Bech32ContractId,
    errors::{error, Result},
    transaction::TxPolicies,
    transaction_builders::{
        Blob, BlobId, BlobTransactionBuilder, CreateTransactionBuilder, TransactionBuilder,
    },
};
pub use load::*;
pub use storage::*;

/// [`Contract`] is a struct to interface with a contract. That includes things such as
/// compiling, deploying, and running transactions against a contract.
#[derive(Debug, Clone)]
pub struct Contract {
    binary: Vec<u8>,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
    contract_id: ContractId,
    code_root: Bytes32,
    state_root: Bytes32,
}

/// Used to control how the contract is going to get split up into blob tx.
pub enum BlobSizePolicy {
    /// Contract chunks can be at most `words` * 8 bytes.
    AtMost { words: usize },
    /// Note: Use a value less than 1.0 (100%):
    /// The theoretical maximum is calculated based on the number of bytes that can fit in a blob transaction
    /// without exceeding the maximum allowed transaction size. This calculation does not account for additional
    /// limiting factors such as:
    /// * network limitations on payload size
    /// * the possibility of the transaction exceeding the maximum gas limit
    /// * the size impact of any inputs/witnesses added to the transaction to cover its fee
    Estimate { percentage_of_theoretical_max: f64 },
}

impl BlobSizePolicy {
    async fn resolve_size(&self, provider: &Provider) -> Result<usize> {
        let size = match self {
            BlobSizePolicy::AtMost { words } => *words * WORD_SIZE,
            BlobSizePolicy::Estimate {
                percentage_of_theoretical_max,
            } => {
                let theoretical_max = BlobTransactionBuilder::default()
                    .estimate_max_blob_size(provider)
                    .await?;

                let percentage_of_theoretical_max =
                    (*percentage_of_theoretical_max * theoretical_max as f64) as usize;

                (percentage_of_theoretical_max / WORD_SIZE) * WORD_SIZE
            }
        };

        if size == 0 {
            Err(error!(Other, "blob size must be greater than 0"))
        } else {
            Ok(size)
        }
    }
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

    /// The contract code has been uploaded in blobs with [`BlobId`]s specified in `blob_ids`. This will create a loader
    /// contract that, when deployed and executed, will load all the specified blobs into memory and delegate the call to the code contained in the blobs.
    pub fn new_loader(
        blob_ids: &[BlobId],
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self> {
        let code = Self::loader_contract(blob_ids)?;
        Ok(Self::new(code, salt, storage_slots))
    }

    /// Splits the contract into blobs, submits them, and awaits confirmation. Then, it deploys a loader contract.
    /// This loader contract will load the blobs into memory and delegate the call to the code contained within the blobs.
    /// This method is useful for deploying large contracts.
    pub async fn deploy_as_loader(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
        blob_size_policy: BlobSizePolicy,
    ) -> Result<Bech32ContractId> {
        let provider = account.try_provider()?;

        let blobs = self.generate_blobs(provider, blob_size_policy).await?;
        let all_blob_ids = blobs.iter().map(|blob| blob.id()).collect::<Vec<_>>();
        let mut already_uploaded = HashSet::new();

        for blob in blobs {
            let id = blob.id();

            if already_uploaded.contains(&id) {
                continue;
            }

            let mut tb = BlobTransactionBuilder::default()
                .with_blob(blob)
                .with_tx_policies(tx_policies)
                .with_max_fee_estimation_tolerance(0.05);

            account.adjust_for_fee(&mut tb, 0).await?;
            account.add_witnesses(&mut tb)?;

            let tx = tb.build(provider).await?;
            provider
                .send_transaction_and_await_commit(tx)
                .await?
                .check(None)?;

            already_uploaded.insert(id);
        }

        Self::new_loader(&all_blob_ids, self.salt, self.storage_slots)?
            .deploy(account, tx_policies)
            .await
    }

    /// Splits the contract binary into blobs based on the size specified by `blob_size_policy`.
    /// This is useful if you prefer to manually deploy the blobs. Once uploaded, you can use [`Contract::new_loader`] to create a loader contract.
    pub async fn generate_blobs(
        &self,
        provider: &Provider,
        policy: BlobSizePolicy,
    ) -> Result<Vec<Blob>> {
        let blob_size = policy.resolve_size(provider).await?;

        let blobs = self
            .binary
            .chunks(blob_size)
            .map(|chunk| Blob::new(chunk.to_vec()))
            .collect();

        Ok(blobs)
    }

    // This function creates a contract that loads the specified blobs into memory and delegates the call to the code contained in the blobs.
    fn loader_contract(blob_ids: &[BlobId]) -> Result<Vec<u8>> {
        const BLOB_ID_SIZE: u16 = 32;
        let get_instructions = |num_of_instructions, num_of_blobs| {
            // There are 2 main steps:
            // 1. Load the blob contents into memory
            // 2. Jump to the beginning of the memory where the blobs were loaded
            // After that the execution continues normally with the loaded contract reading our
            // prepared fn selector and jumps to the selected contract method.
            [
                // 1. load the blob contents into memory
                // find the start of the hardcoded blob ids, which are located after the code ends,
                op::move_(0x10, RegId::IS),
                // 0x10 to hold the address of the current blob id
                op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
                // The contract is going to be loaded from the current value of SP onwards, save
                // the location into 0x16 so we can jump into it later on
                op::move_(0x16, RegId::SP),
                // loop counter
                op::movi(0x13, num_of_blobs),
                // LOOP starts here
                // 0x11 to hold the size of the current blob
                op::bsiz(0x11, 0x10),
                // push the blob contents onto the stack
                op::ldc(0x10, 0, 0x11, 1),
                // move on to the next blob
                op::addi(0x10, 0x10, BLOB_ID_SIZE),
                // decrement the loop counter
                op::subi(0x13, 0x13, 1),
                // Jump backwards 3 instructions if the counter has not reached 0
                op::jnzb(0x13, RegId::ZERO, 3),
                // 3. Jump into the memory where the contract is loaded
                // what follows is called _jmp_mem by the sway compiler
                // subtract the address contained in IS because jmp will add it back
                op::sub(0x16, 0x16, RegId::IS),
                // jmp will multiply by 4 so we need to divide to cancel that out
                op::divi(0x16, 0x16, 4),
                // jump to the start of the contract we loaded
                op::jmp(0x16),
            ]
        };

        let num_of_instructions = u16::try_from(get_instructions(0, 0).len())
            .expect("to never have more than u16::MAX instructions");

        let num_of_blobs = u32::try_from(blob_ids.len()).map_err(|_| {
            error!(
                Other,
                "the number of blobs ({}) exceeds the maximum number of blobs supported: {}",
                blob_ids.len(),
                u32::MAX
            )
        })?;

        let instruction_bytes = get_instructions(num_of_instructions, num_of_blobs)
            .into_iter()
            .flat_map(|instruction| instruction.to_bytes());

        let blob_bytes = blob_ids.iter().flatten().copied();

        Ok(instruction_bytes.chain(blob_bytes).collect())
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
        tx_policies: TxPolicies,
    ) -> Result<Bech32ContractId> {
        let mut tb = CreateTransactionBuilder::prepare_contract_deployment(
            self.binary,
            self.contract_id,
            self.state_root,
            self.salt,
            self.storage_slots,
            tx_policies,
        )
        .with_max_fee_estimation_tolerance(0.05);

        account.add_witnesses(&mut tb)?;
        account.adjust_for_fee(&mut tb, 0).await?;

        let provider = account.try_provider()?;

        let tx = tb.build(provider).await?;

        provider
            .send_transaction_and_await_commit(tx)
            .await?
            .check(None)?;

        Ok(self.contract_id.into())
    }

    pub fn load_from(binary_filepath: impl AsRef<Path>, config: LoadConfiguration) -> Result<Self> {
        let binary_filepath = binary_filepath.as_ref();
        validate_path_and_extension(binary_filepath, "bin")?;

        let mut binary = fs::read(binary_filepath).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to read binary: {binary_filepath:?}: {e}"),
            )
        })?;

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

    pub fn salt(&self) -> Salt {
        self.salt
    }

    pub fn storage_slots(&self) -> &[StorageSlot] {
        &self.storage_slots
    }
}

fn autoload_storage_slots(contract_binary: &Path) -> Result<StorageSlots> {
    let storage_file = expected_storage_slots_filepath(contract_binary)
        .ok_or_else(|| error!(Other, "could not determine storage slots file"))?;

    StorageSlots::load_from_file(&storage_file)
                .map_err(|_| error!(Other, "could not autoload storage slots from file: {storage_file:?}. \
                                    Either provide the file or disable autoloading in `StorageConfiguration`"))
}

fn expected_storage_slots_filepath(contract_binary: &Path) -> Option<PathBuf> {
    let dir = contract_binary.parent()?;

    let binary_filename = contract_binary.file_stem()?.to_str()?;

    Some(dir.join(format!("{binary_filename}-storage_slots.json")))
}

#[cfg(test)]
mod tests {
    use fuels_core::types::errors::Error;
    use tempfile::tempdir;

    use super::*;

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
            .expect_err("should have failed because the storage slots file is missing");

        // then
        let storage_slots_path = temp_dir.path().join("my_contract-storage_slots.json");
        let Error::Other(msg) = error else {
            panic!("expected an error of type `Other`");
        };
        assert_eq!(msg, format!("could not autoload storage slots from file: {storage_slots_path:?}. Either provide the file or disable autoloading in `StorageConfiguration`"));
    }

    fn save_slots(slots: &Vec<StorageSlot>, path: &Path) {
        std::fs::write(
            path,
            serde_json::to_string::<Vec<StorageSlot>>(slots).unwrap(),
        )
        .unwrap()
    }
}
