use std::{default::Default, fmt::Debug, path::Path};

use fuel_tx::{Bytes32, ContractId, Salt, StorageSlot};
use fuels_accounts::Account;
use fuels_core::{
    constants::WORD_SIZE,
    error,
    types::{
        bech32::Bech32ContractId,
        errors::Result,
        transaction::TxPolicies,
        transaction_builders::{Blob, CreateTransactionBuilder},
    },
    Configurables,
};

use super::{
    compute_contract_id_and_state_root, validate_path_and_extension, BlobsNotUploaded, Contract,
    Loader, StorageConfiguration,
};

// In a mod so that we eliminate the footgun of getting the private `code` field without applying
// configurables
mod code_types {
    use fuel_asm::{op, Instruction, RegId};
    use fuels_core::{
        constants::WORD_SIZE,
        traits::Signer,
        types::transaction_builders::{Blob, BlobId, BlobTransactionBuilder, TransactionBuilder},
        Configurables,
    };

    #[derive(Debug, Clone, PartialEq)]
    pub struct Regular {
        code: Vec<u8>,
        configurables: Configurables,
    }

    impl Regular {
        pub fn new(code: Vec<u8>, configurables: Configurables) -> Self {
            Self {
                code,
                configurables,
            }
        }

        pub(crate) fn with_code(self, code: Vec<u8>) -> Self {
            Self { code, ..self }
        }

        pub(crate) fn with_configurables(self, configurables: Configurables) -> Self {
            Self {
                configurables,
                ..self
            }
        }

        pub(crate) fn code(&self) -> Vec<u8> {
            let mut code = self.code.clone();
            self.configurables.update_constants_in(&mut code);
            code
        }
    }

    pub struct Executable<State> {
        state: State,
    }

    impl Executable<Regular> {
        pub fn load_from(path: &str) -> Executable<Regular> {
            let code = std::fs::read(path).unwrap();

            Executable {
                state: Regular::new(code, Default::default()),
            }
        }
    }

    pub struct Loader {
        code: Vec<u8>,
        configurables: Configurables,
    }

    impl Executable<Regular> {
        pub fn with_configurables(self, configurables: impl Into<Configurables>) -> Self {
            Executable {
                state: Regular {
                    configurables: configurables.into(),
                    ..self.state
                },
            }
        }

        pub fn code(&self) -> Vec<u8> {
            self.state.code()
        }

        pub fn to_loader(self) -> Executable<Loader> {
            Executable {
                state: Loader {
                    code: self.state.code,
                    configurables: self.state.configurables,
                },
            }
        }
    }

    fn extract_data_offset(binary: &[u8]) -> usize {
        let data_offset: [u8; 8] = binary[8..16].try_into().unwrap();
        u64::from_be_bytes(data_offset) as usize
    }

    fn transform_into_configurable_loader(
        original_binary: Vec<u8>,
        blob_id: &BlobId,
    ) -> fuels_core::types::errors::Result<Vec<u8>> {
        // The final code is going to have this structure:
        // 1. loader instructions
        // 2. blob id
        // 3. length_of_data_section
        // 4. the data_section (updated with configurables as needed)

        let offset = extract_data_offset(&original_binary);

        let data_section = original_binary[offset..].to_vec();

        // update the data_section here as necessary (with configurables)

        let data_section_len = data_section.len();

        const BLOB_ID_SIZE: u16 = 32;
        const REG_ADDRESS_OF_DATA_AFTER_CODE: u8 = 0x10;
        const REG_START_OF_LOADED_CODE: u8 = 0x11;
        const REG_GENERAL_USE: u8 = 0x12;
        const REG_START_OF_DATA_SECTION: u8 = 0x13;
        let get_instructions = |num_of_instructions| {
            // There are 3 main steps:
            // 1. Load the blob content into memory
            // 2. Load the data section right after the blob
            // 3. Jump to the beginning of the memory where the blob was loaded
            [
                // 1. Load the blob content into memory
                // Find the start of the hardcoded blob ID, which is located after the loader code ends.
                op::move_(REG_ADDRESS_OF_DATA_AFTER_CODE, RegId::PC),
                // hold the address of the blob ID.
                op::addi(
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    num_of_instructions * Instruction::SIZE as u16,
                ),
                // The code is going to be loaded from the current value of SP onwards, save
                // the location into REG_START_OF_LOADED_CODE so we can jump into it at the end.
                op::move_(REG_START_OF_LOADED_CODE, RegId::SP),
                // REG_GENERAL_USE to hold the size of the blob.
                op::bsiz(REG_GENERAL_USE, REG_ADDRESS_OF_DATA_AFTER_CODE),
                // Push the blob contents onto the stack.
                op::ldc(REG_ADDRESS_OF_DATA_AFTER_CODE, 0, REG_GENERAL_USE, 1),
                // Move on to the data section length
                op::addi(
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    BLOB_ID_SIZE,
                ),
                // load the size of the data section into REG_GENERAL_USE
                op::lw(REG_GENERAL_USE, REG_ADDRESS_OF_DATA_AFTER_CODE, 0),
                // after we have read the length of the data section, we move the pointer to the actual
                // data by skipping WORD_SIZE B.
                op::addi(
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    WORD_SIZE as u16,
                ),
                // extend the stack
                op::cfe(REG_GENERAL_USE),
                // move to the start of the newly allocated stack
                op::sub(REG_START_OF_DATA_SECTION, RegId::SP, REG_GENERAL_USE),
                // load the data section onto the stack
                op::mcp(
                    REG_START_OF_DATA_SECTION,
                    REG_ADDRESS_OF_DATA_AFTER_CODE,
                    REG_GENERAL_USE,
                ),
                // Jump into the memory where the contract is loaded.
                // What follows is called _jmp_mem by the sway compiler.
                // Subtract the address contained in IS because jmp will add it back.
                op::sub(
                    REG_START_OF_LOADED_CODE,
                    REG_START_OF_LOADED_CODE,
                    RegId::IS,
                ),
                // jmp will multiply by 4, so we need to divide to cancel that out.
                op::divi(REG_START_OF_LOADED_CODE, REG_START_OF_LOADED_CODE, 4),
                // Jump to the start of the contract we loaded.
                op::jmp(REG_START_OF_LOADED_CODE),
            ]
        };

        let num_of_instructions = u16::try_from(get_instructions(0).len())
            .expect("to never have more than u16::MAX instructions");

        let instruction_bytes = get_instructions(num_of_instructions)
            .into_iter()
            .flat_map(|instruction| instruction.to_bytes());

        let blob_bytes = blob_id.iter().copied();

        Ok(instruction_bytes
            .chain(blob_bytes)
            .chain(data_section_len.to_be_bytes())
            .chain(data_section)
            .collect())
    }

    impl Executable<Loader> {
        pub fn with_configurables(self, configurables: impl Into<Configurables>) -> Self {
            Executable {
                state: Loader {
                    configurables: configurables.into(),
                    ..self.state
                },
            }
        }

        pub fn code(&self) -> Vec<u8> {
            let mut code = self.state.code.clone();

            self.state.configurables.update_constants_in(&mut code);

            let blob_id = self.blob().id();

            transform_into_configurable_loader(code, &blob_id).unwrap()
        }

        pub fn blob(&self) -> Blob {
            // TODO: check bounds
            let data_section_offset = extract_data_offset(&self.state.code);

            let code_without_data_section = self.state.code[..data_section_offset].to_vec();

            Blob::new(code_without_data_section)
        }

        pub async fn upload_blob<A: fuels_accounts::Account + Signer + Clone>(&self, account: A) {
            let blob = self.blob();

            let mut tb = BlobTransactionBuilder::default().with_blob(blob);

            account.adjust_for_fee(&mut tb, 0).await.unwrap();

            tb.add_signer(account.clone()).unwrap();

            let provider = account.try_provider().unwrap();
            let tx = tb.build(provider).await.unwrap();

            provider
                .send_transaction_and_await_commit(tx)
                .await
                .unwrap()
                .check(None)
                .unwrap();
        }
    }
}
pub use code_types::*;

impl Contract<Regular> {
    pub fn with_code(self, code: Vec<u8>) -> Self {
        Self {
            code: self.code.with_code(code),
            salt: self.salt,
            storage_slots: self.storage_slots,
        }
    }

    pub fn with_configurables(self, configurables: impl Into<Configurables>) -> Self {
        Self {
            code: self.code.with_configurables(configurables.into()),
            ..self
        }
    }

    pub fn code(&self) -> Vec<u8> {
        self.code.code()
    }

    pub fn contract_id(&self) -> ContractId {
        self.compute_roots().0
    }

    pub fn code_root(&self) -> Bytes32 {
        self.compute_roots().1
    }

    pub fn state_root(&self) -> Bytes32 {
        self.compute_roots().2
    }

    fn compute_roots(&self) -> (ContractId, Bytes32, Bytes32) {
        compute_contract_id_and_state_root(&self.code(), &self.salt, &self.storage_slots)
    }

    /// Loads a contract from a binary file. Salt and storage slots are loaded as well, depending on the configuration provided.
    pub fn load_from(
        binary_filepath: impl AsRef<Path>,
        config: LoadConfiguration,
    ) -> Result<Contract<Regular>> {
        let binary_filepath = binary_filepath.as_ref();
        validate_path_and_extension(binary_filepath, "bin")?;

        let binary = std::fs::read(binary_filepath).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("failed to read binary: {binary_filepath:?}: {e}"),
            )
        })?;

        let storage_slots = super::determine_storage_slots(config.storage, binary_filepath)?;

        Ok(Contract {
            code: Regular::new(binary, config.configurables),
            salt: config.salt,
            storage_slots,
        })
    }

    /// Creates a regular contract with the given code, salt, and storage slots.
    pub fn regular(
        code: Vec<u8>,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
    ) -> Contract<Regular> {
        Contract {
            code: Regular::new(code, Configurables::default()),
            salt,
            storage_slots,
        }
    }

    /// Deploys a compiled contract to a running node.
    /// To deploy a contract, you need an account with enough assets to pay for deployment.
    /// This account will also receive the change.
    pub async fn deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
    ) -> Result<Bech32ContractId> {
        let contract_id = self.contract_id();
        let state_root = self.state_root();
        let salt = self.salt;
        let storage_slots = self.storage_slots;

        let mut tb = CreateTransactionBuilder::prepare_contract_deployment(
            self.code.code(),
            contract_id,
            state_root,
            salt,
            storage_slots.to_vec(),
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

        Ok(contract_id.into())
    }

    /// Converts a regular contract into a loader contract, splitting the code into blobs.
    pub fn convert_to_loader(
        self,
        max_words_per_blob: usize,
    ) -> Result<Contract<Loader<BlobsNotUploaded>>> {
        if max_words_per_blob == 0 {
            return Err(error!(Other, "blob size must be greater than 0"));
        }
        let blobs = self
            .code()
            .chunks(max_words_per_blob.saturating_mul(WORD_SIZE))
            .map(|chunk| Blob::new(chunk.to_vec()))
            .collect();

        Contract::loader_from_blobs(blobs, self.salt, self.storage_slots)
    }

    /// Deploys the contract either as a regular contract or as a loader contract if it exceeds the size limit.
    pub async fn smart_deploy(
        self,
        account: &impl Account,
        tx_policies: TxPolicies,
        max_words_per_blob: usize,
    ) -> Result<Bech32ContractId> {
        let provider = account.try_provider()?;
        let max_contract_size = provider
            .consensus_parameters()
            .contract_params()
            .contract_max_size() as usize;

        if self.code().len() <= max_contract_size {
            self.deploy(account, tx_policies).await
        } else {
            self.convert_to_loader(max_words_per_blob)?
                .deploy(account, tx_policies)
                .await
        }
    }
}

/// Configuration for contract deployment.
#[derive(Debug, Clone, Default)]
pub struct LoadConfiguration {
    pub(crate) storage: StorageConfiguration,
    pub(crate) configurables: Configurables,
    pub(crate) salt: Salt,
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
