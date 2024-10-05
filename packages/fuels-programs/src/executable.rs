use fuel_asm::{op, Instruction, RegId};
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        errors::Result,
        transaction_builders::{Blob, BlobId, BlobTransactionBuilder},
    },
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
}

pub struct Executable<State> {
    state: State,
}

impl Executable<Regular> {
    pub fn from_bytes(code: Vec<u8>) -> Self {
        Executable {
            state: Regular::new(code, Default::default()),
        }
    }

    pub fn load_from(path: &str) -> Result<Executable<Regular>> {
        let code = std::fs::read(path)?;

        Ok(Executable {
            state: Regular::new(code, Default::default()),
        })
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
        let mut code = self.state.code.clone();
        self.state.configurables.update_constants_in(&mut code);
        code
    }

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
    // The final code is going to have this structure:
    // 1. loader instructions
    // 2. blob id
    // 3. length_of_data_section
    // 4. the data_section (updated with configurables as needed)

    let offset = extract_data_offset(&binary)?;

    if binary.len() <= offset {
        return Err(fuels_core::error!(
            Other,
            "data section offset is out of bounds, offset: {offset}, binary len: {}",
            binary.len()
        ));
    }

    let data_section = binary[offset..].to_vec();

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

fn validate_loader_can_be_made_from_code(
    mut code: Vec<u8>,
    configurables: Configurables,
) -> Result<()> {
    configurables.update_constants_in(&mut code);

    // BlobId currently doesn't affect our ability to produce the loader code
    transform_into_configurable_loader(code, &Default::default())?;

    Ok(())
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

        transform_into_configurable_loader(code, &blob_id)
            .expect("checked before turning into a Executable<Loader>")
    }

    pub fn blob(&self) -> Blob {
        let data_section_offset = extract_data_offset(&self.state.code)
            .expect("checked before turning into a Executable<Loader>");

        let code_without_data_section = self.state.code[..data_section_offset].to_vec();

        Blob::new(code_without_data_section)
    }

    pub async fn upload_blob(&self, account: impl fuels_accounts::Account) -> Result<()> {
        let blob = self.blob();
        let provider = account.try_provider()?;

        // TODO: use more optimal endpoint once it is made available in fuel-core-client
        if provider.blob(blob.id()).await?.is_some() {
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
