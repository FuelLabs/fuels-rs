use fuel_asm::{op, Instruction, RegId};
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        errors::Result,
        transaction_builders::{Blob, BlobId, BlobTransactionBuilder},
    },
    Configurables,
};

/// This struct represents a standard executable with its associated bytecode and configurables.
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
}

/// Used to transform Script or Predicate code into a loader variant, where the code is uploaded as
/// a blob and the binary itself is substituted with code that will load the blob code and apply
/// the given configurables to the Script/Predicate.
pub struct Executable<State> {
    state: State,
}

impl Executable<Regular> {
    pub fn from_bytes(code: Vec<u8>) -> Self {
        Executable {
            state: Regular::new(code, Default::default()),
        }
    }

    /// Loads an `Executable<Regular>` from a file at the given path.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to load the executable from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Executable<Regular>` or an error if loading fails.
    pub fn load_from(path: &str) -> Result<Executable<Regular>> {
        let code = std::fs::read(path)?;

        Ok(Executable {
            state: Regular::new(code, Default::default()),
        })
    }

    pub fn with_configurables(self, configurables: impl Into<Configurables>) -> Self {
        Executable {
            state: Regular {
                configurables: configurables.into(),
                ..self.state
            },
        }
    }

    /// Returns the code of the executable with configurables applied.
    ///
    /// # Returns
    ///
    /// The bytecode of the executable with configurables updated.
    pub fn code(&self) -> Vec<u8> {
        let mut code = self.state.code.clone();
        self.state.configurables.update_constants_in(&mut code);
        code
    }

    /// Converts the `Executable<Regular>` into an `Executable<Loader>`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Executable<Loader>` or an error if loader code cannot be
    /// generated for the given binary.
    pub fn convert_to_loader(self) -> Result<Executable<Loader>> {
        validate_loader_can_be_made_from_code(
            self.state.code.clone(),
            self.state.configurables.clone(),
        )?;

        Ok(Executable {
            state: Loader {
                code: self.state.code,
                configurables: self.state.configurables,
            },
        })
    }
}

pub struct Loader {
    code: Vec<u8>,
    configurables: Configurables,
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

    /// Returns the code of the loader executable with configurables applied.
    pub fn code(&self) -> Vec<u8> {
        let mut code = self.state.code.clone();

        self.state.configurables.update_constants_in(&mut code);

        let blob_id = self.blob().id();

        transform_into_configurable_loader(code, &blob_id)
            .expect("checked before turning into a Executable<Loader>")
    }

    /// A Blob containing the original executable code minus the data section.
    pub fn blob(&self) -> Blob {
        let data_section_offset = extract_data_offset(&self.state.code)
            .expect("checked before turning into a Executable<Loader>");

        let code_without_data_section = self.state.code[..data_section_offset].to_vec();

        Blob::new(code_without_data_section)
    }

    /// Uploads a blob containing the original executable code minus the data section.
    pub async fn upload_blob(&self, account: impl fuels_accounts::Account) -> Result<()> {
        let blob = self.blob();
        let provider = account.try_provider()?;

        if provider.blob_exists(blob.id()).await? {
            return Ok(());
        }

        let mut tb = BlobTransactionBuilder::default().with_blob(self.blob());

        account.adjust_for_fee(&mut tb, 0).await?;

        account.add_witnesses(&mut tb)?;

        let tx = tb.build(provider).await?;

        provider
            .send_transaction_and_await_commit(tx)
            .await?
            .check(None)?;

        Ok(())
    }
}

fn extract_data_offset(binary: &[u8]) -> Result<usize> {
    if binary.len() < 16 {
        return Err(fuels_core::error!(
            Other,
            "given binary is too short to contain a data offset, len: {}",
            binary.len()
        ));
    }

    let data_offset: [u8; 8] = binary[8..16].try_into().expect("checked above");

    Ok(u64::from_be_bytes(data_offset) as usize)
}

fn transform_into_configurable_loader(binary: Vec<u8>, blob_id: &BlobId) -> Result<Vec<u8>> {
    // The final code is going to have this structure (if the data section is non-empty):
    // 1. loader instructions
    // 2. blob id
    // 3. length_of_data_section
    // 4. the data_section (updated with configurables as needed)
    const BLOB_ID_SIZE: u16 = 32;
    const REG_ADDRESS_OF_DATA_AFTER_CODE: u8 = 0x10;
    const REG_START_OF_LOADED_CODE: u8 = 0x11;
    const REG_GENERAL_USE: u8 = 0x12;
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
            // load the data section of the executable
            op::ldc(REG_ADDRESS_OF_DATA_AFTER_CODE, 0, REG_GENERAL_USE, 2),
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

    let get_instructions_no_data_section = |num_of_instructions| {
        // There are 2 main steps:
        // 1. Load the blob content into memory
        // 2. Jump to the beginning of the memory where the blob was loaded
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

    let offset = extract_data_offset(&binary)?;

    if binary.len() < offset {
        return Err(fuels_core::error!(
            Other,
            "data section offset is out of bounds, offset: {offset}, binary len: {}",
            binary.len()
        ));
    }

    let data_section = binary[offset..].to_vec();

    if !data_section.is_empty() {
        let num_of_instructions = u16::try_from(get_instructions(0).len())
            .expect("to never have more than u16::MAX instructions");

        let instruction_bytes = get_instructions(num_of_instructions)
            .into_iter()
            .flat_map(|instruction| instruction.to_bytes());

        let blob_bytes = blob_id.iter().copied();

        Ok(instruction_bytes
            .chain(blob_bytes)
            .chain(data_section.len().to_be_bytes())
            .chain(data_section)
            .collect())
    } else {
        let num_of_instructions = u16::try_from(get_instructions_no_data_section(0).len())
            .expect("to never have more than u16::MAX instructions");

        let instruction_bytes = get_instructions_no_data_section(num_of_instructions)
            .into_iter()
            .flat_map(|instruction| instruction.to_bytes());

        let blob_bytes = blob_id.iter().copied();

        Ok(instruction_bytes.chain(blob_bytes).collect())
    }
}

fn validate_loader_can_be_made_from_code(
    mut code: Vec<u8>,
    configurables: Configurables,
) -> Result<()> {
    configurables.update_constants_in(&mut code);

    // BlobId currently doesn't affect our ability to produce the loader code
    transform_into_configurable_loader(code, &Default::default())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_core::Configurables;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_executable_regular_from_bytes() {
        // Given: Some bytecode
        let code = vec![1u8, 2, 3, 4];

        // When: Creating an Executable<Regular> from bytes
        let executable = Executable::<Regular>::from_bytes(code.clone());

        // Then: The executable should have the given code and default configurables
        assert_eq!(executable.state.code, code);
        assert_eq!(executable.state.configurables, Default::default());
    }

    #[test]
    fn test_executable_regular_load_from() {
        // Given: A temporary file containing some bytecode
        let code = vec![5u8, 6, 7, 8];
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(&code)
            .expect("Failed to write to temp file");
        let path = temp_file.path().to_str().unwrap();

        // When: Loading an Executable<Regular> from the file
        let executable_result = Executable::<Regular>::load_from(path);

        // Then: The executable should be created successfully with the correct code
        assert!(executable_result.is_ok());
        let executable = executable_result.unwrap();
        assert_eq!(executable.state.code, code);
        assert_eq!(executable.state.configurables, Default::default());
    }

    #[test]
    fn test_executable_regular_load_from_invalid_path() {
        // Given: An invalid file path
        let invalid_path = "/nonexistent/path/to/file";

        // When: Attempting to load an Executable<Regular> from the invalid path
        let executable_result = Executable::<Regular>::load_from(invalid_path);

        // Then: The operation should fail with an error
        assert!(executable_result.is_err());
    }

    #[test]
    fn test_executable_regular_with_configurables() {
        // Given: An Executable<Regular> and some configurables
        let code = vec![1u8, 2, 3, 4];
        let executable = Executable::<Regular>::from_bytes(code);
        let configurables = Configurables::new(vec![(2, vec![1])]);

        // When: Setting new configurables
        let new_executable = executable.with_configurables(configurables.clone());

        // Then: The executable should have the new configurables
        assert_eq!(new_executable.state.configurables, configurables);
    }

    #[test]
    fn test_executable_regular_code() {
        // Given: An Executable<Regular> with some code and configurables
        let code = vec![1u8, 2, 3, 4];
        let configurables = Configurables::new(vec![(1, vec![1])]);
        let executable =
            Executable::<Regular>::from_bytes(code.clone()).with_configurables(configurables);

        // When: Retrieving the code after applying configurables
        let modified_code = executable.code();

        assert_eq!(modified_code, vec![1, 1, 3, 4]);
    }

    #[test]
    fn test_loader_extracts_code_and_data_section_correctly() {
        // Given: An Executable<Regular> with valid code
        let padding = vec![0; 8];
        let offset = 20u64.to_be_bytes().to_vec();
        let some_random_instruction = vec![1, 2, 3, 4];
        let data_section = vec![5, 6, 7, 8];
        let code = [
            padding.clone(),
            offset.clone(),
            some_random_instruction.clone(),
            data_section,
        ]
        .concat();
        let executable = Executable::<Regular>::from_bytes(code.clone());

        // When: Converting to a loader
        let loader = executable.convert_to_loader().unwrap();

        let blob = loader.blob();
        let data_stripped_code = [padding, offset, some_random_instruction].concat();
        assert_eq!(blob.as_ref(), data_stripped_code);

        let loader_code = loader.code();
        let blob_id = blob.id();
        assert_eq!(
            loader_code,
            transform_into_configurable_loader(code, &blob_id).unwrap()
        )
    }

    #[test]
    fn test_executable_regular_convert_to_loader_with_invalid_code() {
        // Given: An Executable<Regular> with invalid code (too short)
        let code = vec![1u8, 2]; // Insufficient length for a valid data offset
        let executable = Executable::<Regular>::from_bytes(code);

        // When: Attempting to convert to a loader
        let result = executable.convert_to_loader();

        // Then: The conversion should fail with an error
        assert!(result.is_err());
    }

    #[test]
    fn executable_with_no_data_section() {
        // to skip over the first 2 half words and skip over the offset itself, basically stating
        // that there is no data section
        let data_section_offset = 16u64;

        let code = [vec![0; 8], data_section_offset.to_be_bytes().to_vec()].concat();

        Executable::from_bytes(code).convert_to_loader().unwrap();
    }
}
