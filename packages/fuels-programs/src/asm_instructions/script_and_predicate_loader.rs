use fuel_asm::{op, Instruction, RegId};
use fuels_core::constants::WORD_SIZE;

struct LoaderInstructions {
    instructions: Vec<Instruction>,
    blob_id: [u8; 32],
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
