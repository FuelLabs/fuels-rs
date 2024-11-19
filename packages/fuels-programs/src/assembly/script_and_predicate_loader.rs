use fuel_asm::{op, Instruction, RegId};
use fuels_core::{constants::WORD_SIZE, types::errors::Result};
use itertools::Itertools;

use crate::assembly::cursor::WasmFriendlyCursor;

pub struct LoaderCode {
    blob_id: [u8; 32],
    code: Vec<u8>,
    data_offset: usize,
}

impl LoaderCode {
    // std gated because of Blob usage which is in transaction_builders which are currently not
    // nostd friendly
    #[cfg(feature = "std")]
    pub fn from_normal_binary(binary: Vec<u8>) -> Result<Self> {
        let (original_code, data_section) = split_at_data_offset(&binary)?;

        let blob_id =
            fuels_core::types::transaction_builders::Blob::from(original_code.to_vec()).id();
        let (loader_code, data_offset) = Self::generate_loader_code(blob_id, data_section);

        Ok(Self {
            blob_id,
            code: loader_code,
            data_offset,
        })
    }

    pub fn from_loader_binary(binary: &[u8]) -> Result<Option<Self>> {
        if let Some((blob_id, data_section_offset)) = extract_blob_id_and_data_offset(binary)? {
            Ok(Some(Self {
                data_offset: data_section_offset,
                code: binary.to_vec(),
                blob_id,
            }))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn extract_blob(binary: &[u8]) -> Result<fuels_core::types::transaction_builders::Blob> {
        let (code, _) = split_at_data_offset(binary)?;
        Ok(code.to_vec().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.code
    }

    pub fn data_section_offset(&self) -> usize {
        self.data_offset
    }

    fn generate_loader_code(blob_id: [u8; 32], data_section: &[u8]) -> (Vec<u8>, usize) {
        if !data_section.is_empty() {
            generate_loader_w_data_section(blob_id, data_section)
        } else {
            generate_loader_wo_data_section(blob_id)
        }
    }

    pub fn blob_id(&self) -> [u8; 32] {
        self.blob_id
    }
}

fn extract_blob_id_and_data_offset(binary: &[u8]) -> Result<Option<([u8; 32], usize)>> {
    let (has_data_section, mut cursor) =
        if let Some(cursor) = consume_instructions(binary, &loader_instructions_w_data_section()) {
            (true, cursor)
        } else if let Some(cursor) =
            consume_instructions(binary, &loader_instructions_no_data_section())
        {
            (false, cursor)
        } else {
            return Ok(None);
        };

    let blob_id = cursor.consume_fixed("blob id")?;
    if has_data_section {
        let _data_section_len = cursor.consume(WORD_SIZE, "data section len")?;
    }

    let data_section_offset = binary
        .len()
        .checked_sub(cursor.unconsumed())
        .expect("must be less or eq");

    Ok(Some((blob_id, data_section_offset)))
}

fn consume_instructions<'a>(
    binary: &'a [u8],
    expected_instructions: &[Instruction],
) -> Option<WasmFriendlyCursor<'a>> {
    let loader_instructions_byte_size = expected_instructions.len() * Instruction::SIZE;

    let mut script_cursor = WasmFriendlyCursor::new(binary);
    let instruction_bytes = script_cursor
        .consume(loader_instructions_byte_size, "loader instructions")
        .ok()?;

    let instructions = fuel_asm::from_bytes(instruction_bytes.to_vec())
        .collect::<std::result::Result<Vec<Instruction>, _>>()
        .ok()?;

    instructions
        .iter()
        .zip(expected_instructions.iter())
        .all(|(actual, expected)| actual == expected)
        .then_some(script_cursor)
}

fn generate_loader_wo_data_section(blob_id: [u8; 32]) -> (Vec<u8>, usize) {
    let instruction_bytes = loader_instructions_no_data_section()
        .into_iter()
        .flat_map(|instruction| instruction.to_bytes());

    let code = instruction_bytes
        .chain(blob_id.iter().copied())
        .collect_vec();
    // there is no data section, so we point the offset to the end of the file
    let new_data_section_offset = code.len();

    (code, new_data_section_offset)
}

fn generate_loader_w_data_section(blob_id: [u8; 32], data_section: &[u8]) -> (Vec<u8>, usize) {
    // The final code is going to have this structure:
    // 1. loader instructions
    // 2. blob id
    // 3. length_of_data_section
    // 4. the data_section (updated with configurables as needed)

    let instruction_bytes = loader_instructions_w_data_section()
        .into_iter()
        .flat_map(|instruction| instruction.to_bytes())
        .collect_vec();

    let blob_bytes = blob_id.iter().copied().collect_vec();

    let original_data_section_len_encoded = u64::try_from(data_section.len())
        .expect("data section to be less than u64::MAX")
        .to_be_bytes();

    // The data section is placed after all of the instructions, the BlobId, and the number representing
    // how big the data section is.
    let new_data_section_offset =
        instruction_bytes.len() + blob_bytes.len() + original_data_section_len_encoded.len();

    let code = instruction_bytes
        .into_iter()
        .chain(blob_bytes)
        .chain(original_data_section_len_encoded)
        .chain(data_section.to_vec())
        .collect();

    (code, new_data_section_offset)
}

fn loader_instructions_no_data_section() -> [Instruction; 8] {
    const REG_ADDRESS_OF_DATA_AFTER_CODE: u8 = 0x10;
    const REG_START_OF_LOADED_CODE: u8 = 0x11;
    const REG_GENERAL_USE: u8 = 0x12;

    const NUM_OF_INSTRUCTIONS: u16 = 8;

    // There are 2 main steps:
    // 1. Load the blob content into memory
    // 2. Jump to the beginning of the memory where the blob was loaded
    let instructions = [
        // 1. Load the blob content into memory
        // Find the start of the hardcoded blob ID, which is located after the loader code ends.
        op::move_(REG_ADDRESS_OF_DATA_AFTER_CODE, RegId::PC),
        // hold the address of the blob ID.
        op::addi(
            REG_ADDRESS_OF_DATA_AFTER_CODE,
            REG_ADDRESS_OF_DATA_AFTER_CODE,
            NUM_OF_INSTRUCTIONS * Instruction::SIZE as u16,
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
    ];

    debug_assert_eq!(instructions.len(), NUM_OF_INSTRUCTIONS as usize);

    instructions
}

pub fn loader_instructions_w_data_section() -> [Instruction; 12] {
    const BLOB_ID_SIZE: u16 = 32;
    const REG_ADDRESS_OF_DATA_AFTER_CODE: u8 = 0x10;
    const REG_START_OF_LOADED_CODE: u8 = 0x11;
    const REG_GENERAL_USE: u8 = 0x12;

    // extract the length of the NoDataSectionLoaderInstructions type
    const NUM_OF_INSTRUCTIONS: u16 = 12;

    // There are 3 main steps:
    // 1. Load the blob content into memory
    // 2. Load the data section right after the blob
    // 3. Jump to the beginning of the memory where the blob was loaded
    let instructions = [
        // 1. Load the blob content into memory
        // Find the start of the hardcoded blob ID, which is located after the loader code ends.
        op::move_(REG_ADDRESS_OF_DATA_AFTER_CODE, RegId::PC),
        // hold the address of the blob ID.
        op::addi(
            REG_ADDRESS_OF_DATA_AFTER_CODE,
            REG_ADDRESS_OF_DATA_AFTER_CODE,
            NUM_OF_INSTRUCTIONS * Instruction::SIZE as u16,
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
    ];

    debug_assert_eq!(instructions.len(), NUM_OF_INSTRUCTIONS as usize);

    instructions
}

pub fn extract_data_offset(binary: &[u8]) -> Result<usize> {
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

pub fn split_at_data_offset(binary: &[u8]) -> Result<(&[u8], &[u8])> {
    let offset = extract_data_offset(binary)?;
    if binary.len() < offset {
        return Err(fuels_core::error!(
            Other,
            "data section offset is out of bounds, offset: {offset}, binary len: {}",
            binary.len()
        ));
    }

    Ok(binary.split_at(offset))
}
