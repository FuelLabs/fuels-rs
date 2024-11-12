use fuels_core::error;

use fuel_asm::{op, Instruction, RegId};
use fuels_core::{
    constants::WORD_SIZE,
    types::{
        errors::Result,
        transaction_builders::{Blob, BlobId},
    },
};
use itertools::Itertools;

use crate::asm_instructions::cursor::WasmFriendlyCursor;

pub struct LoaderCode {
    data_section: Vec<u8>,
    blob_id: BlobId,
}

impl LoaderCode {
    pub fn from_normal_binary(binary: Vec<u8>) -> Result<Self> {
        let (code, data_section) = split_at_data_offset(&binary)?;
        Ok(Self {
            data_section: data_section.to_owned(),
            blob_id: Blob::from(code.to_vec()).id(),
        })
    }

    pub fn from_loader_binary(binary: &[u8]) -> Result<Option<Self>> {
        let expected_loader_instructions = loader_instructions();
        let loader_instructions_byte_size = expected_loader_instructions.len() * Instruction::SIZE;

        let mut script_cursor = WasmFriendlyCursor::new(binary);

        // we give up if we don't have enough instructions or if they cannot be decoded. Undecodable
        // "instructions"" are present in a standard sway script in the form of the data offset in the
        // second word of the binary
        let Ok(instruction_bytes) =
            script_cursor.consume(loader_instructions_byte_size, "loader instructions")
        else {
            return Ok(None);
        };

        let Some(instructions) = fuel_asm::from_bytes(instruction_bytes.to_vec())
            .collect::<std::result::Result<Vec<Instruction>, _>>()
            .ok()
        else {
            return Ok(None);
        };

        if instructions
            .iter()
            .zip(expected_loader_instructions.iter())
            .any(|(actual, expected)| actual != expected)
        {
            return Ok(None);
        }

        let blob_id = script_cursor.consume_fixed("blob id")?;

        let _data_section_len = script_cursor.consume(WORD_SIZE, "data section len")?;

        Ok(Some(Self {
            data_section: script_cursor.consume_all().to_vec(),
            blob_id,
        }))
    }

    pub fn extract_blob(binary: &[u8]) -> Result<Blob> {
        let (code, _) = split_at_data_offset(binary)?;
        Ok(Blob::from(code.to_vec()))
    }

    pub fn to_bytes_w_offset(&self) -> (Vec<u8>, usize) {
        // The final code is going to have this structure (if the data section is non-empty):
        // 1. loader instructions
        // 2. blob id
        // 3. length_of_data_section
        // 4. the data_section (updated with configurables as needed)

        if !self.data_section.is_empty() {
            let instruction_bytes = loader_instructions()
                .into_iter()
                .flat_map(|instruction| instruction.to_bytes())
                .collect_vec();

            let blob_bytes = self.blob_id.iter().copied().collect_vec();

            let original_data_section_len_encoded = u64::try_from(self.data_section.len())
                .expect("data section to be less than u64::MAX")
                .to_be_bytes();

            // The data section is placed after all of the instructions, the BlobId, and the number representing
            // how big the data section is.
            let new_data_section_offset = instruction_bytes.len()
                + blob_bytes.len()
                + original_data_section_len_encoded.len();

            let code = instruction_bytes
                .into_iter()
                .chain(blob_bytes)
                .chain(original_data_section_len_encoded)
                .chain(self.data_section.clone())
                .collect();

            (code, new_data_section_offset)
        } else {
            let instruction_bytes = loader_instructions_no_data_section()
                .into_iter()
                .flat_map(|instruction| instruction.to_bytes());

            let code = instruction_bytes
                .chain(self.blob_id.iter().copied())
                .collect_vec();
            // there is no data section, so we point the offset to the end of the file
            let new_data_section_offset = code.len();

            (code, new_data_section_offset)
        }
    }

    pub fn blob_id(&self) -> [u8; 32] {
        self.blob_id
    }
}

pub(crate) fn loader_instructions_no_data_section() -> [Instruction; 8] {
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

pub(crate) fn loader_instructions() -> [Instruction; 12] {
    const BLOB_ID_SIZE: u16 = 32;
    const REG_ADDRESS_OF_DATA_AFTER_CODE: u8 = 0x10;
    const REG_START_OF_LOADED_CODE: u8 = 0x11;
    const REG_GENERAL_USE: u8 = 0x12;

    // extract the lenght of the NoDataSectionLoaderInstructions type
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

pub(crate) fn extract_data_offset(binary: &[u8]) -> Result<usize> {
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

pub(crate) fn split_at_data_offset(binary: &[u8]) -> Result<(&[u8], &[u8])> {
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
