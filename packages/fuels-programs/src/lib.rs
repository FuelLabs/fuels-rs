use fuel_asm::{op, Instruction, RegId};

pub mod calls;
pub mod contract;
pub mod responses;

// This is the contract from the tooling team
// configurable {
//     TARGET_1: ContractId = ContractId::from(0x729ec21b3966e9105699aa6f10c07bec8af0b72c6fadd099961d8cfbea34e45f),
//     TARGET_2: ContractId = ContractId::from(0x91fce82e763bbb94c788510a9249cb501460a53c9fe27a68365cee70b5bd6de2),
//     TARGET_3: ContractId = ContractId::from(0x5c655e1c02612ea7743b857816226faee077bb357dc6966a234f1db5fca3c33f),
//     TARGET_4: ContractId = ContractId::from(0x2b0a8a8fde26d345c5ec441fec7ab62fa080b0e1a0a7bf2817f0a6dcef827eda),
// }
//
// abi RunExternalTest {
//     fn test_function() -> bool;
// }
//
// impl RunExternalTest for Contract {
//     fn test_function() -> bool {
//         run_external4(TARGET_1, TARGET_2, TARGET_3, TARGET_4)
//     }
// }
//
// fn run_external4(load_target1: ContractId, load_target2: ContractId, load_target3: ContractId, load_target4: ContractId) -> ! {
//     asm(
//         load_target1: load_target1,
//         load_target2: load_target2,
//         load_target3: load_target3,
//         load_target4: load_target4,
//         load_target2_heap,
//         load_target3_heap,
//         load_target4_heap,
//         heap_alloc_size,
//         length1,
//         length2,
//         length3,
//         length4,
//         ssp_saved,
//         cur_stack_size,
//     ) {
//         csiz length1 load_target1;
//         csiz length2 load_target2;
//         csiz length3 load_target3;
//         csiz length4 load_target4;
//         addi heap_alloc_size zero i32;
//         aloc heap_alloc_size;
//         mcp hp load_target2 heap_alloc_size;
//         move load_target2_heap hp;
//         addi heap_alloc_size zero i32;
//         aloc heap_alloc_size;
//         mcp hp load_target3 heap_alloc_size;
//         move load_target3_heap hp;
//         addi heap_alloc_size zero i32;
//         aloc heap_alloc_size;
//         mcp hp load_target4 heap_alloc_size;
//         move load_target4_heap hp;
//         move ssp_saved ssp;
//         sub cur_stack_size sp ssp;
//         cfs cur_stack_size;
//         ldc load_target1 zero length1;
//         ldc load_target2_heap zero length2;
//         ldc load_target3_heap zero length3;
//         ldc load_target4_heap zero length4;
//         addi heap_alloc_size zero i64;
//         aloc heap_alloc_size;
//         sw hp ssp_saved i0;
//     }
//     __jmp_mem()
// }
pub fn loader_contract(blob_ids: &[[u8; 32]]) -> Vec<u8> {
    const BLOB_ID_SIZE: u16 = 32;
    let get_instructions = |num_of_instructions, num_of_blobs| {
        [
            // 0x12 is going to hold the total size of the contract
            op::move_(0x12, RegId::ZERO),
            // find the start of the hardcoded blob ids, which are located after the code ends
            op::move_(0x10, RegId::IS),
            // 0x10 to hold the address of the current blob id
            op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
            // loop counter
            op::addi(0x13, RegId::ZERO, num_of_blobs),
            // LOOP starts here
            // 0x11 to hold the size of the current blob
            op::bsiz(0x11, 0x10),
            // update the total size of the contract
            op::add(0x12, 0x12, 0x11),
            // move on to the next blob
            op::addi(0x10, 0x10, BLOB_ID_SIZE),
            // decrement the loop counter
            op::subi(0x13, 0x13, 1),
            // Jump backwards 3 instructions if the counter has not reached 0
            op::jneb(0x13, RegId::ZERO, RegId::ZERO, 3),
            // move the stack pointer by the contract size since we need to write the contract on the stack
            op::cfe(0x12),
            // find the start of the hardcoded blob ids, which are located after the code ends
            op::move_(0x10, RegId::IS),
            // 0x10 to hold the address of the current blob id
            op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
            // 0x12 is going to hold the total bytes loaded of the contract
            op::move_(0x12, RegId::ZERO),
            // loop counter
            op::addi(0x13, RegId::ZERO, num_of_blobs),
            // LOOP starts here
            // 0x11 to hold the size of the current blob
            op::bsiz(0x11, 0x10),
            // the location where to load the current blob (start of stack)
            op::move_(0x14, RegId::SSP),
            // move to where this blob should be loaded by adding the total bytes loaded
            op::add(0x14, 0x14, 0x12),
            // load the current blob
            op::bldd(0x14, 0x10, RegId::ZERO, 0x11),
            // update the total bytes loaded
            op::add(0x12, 0x12, 0x11),
            // move on to the next blob
            op::addi(0x10, 0x10, BLOB_ID_SIZE),
            // decrement the loop counter
            op::subi(0x13, 0x13, 1),
            // Jump backwards 6 instructions if the counter has not reached 0
            op::jneb(0x13, RegId::ZERO, RegId::ZERO, 6),
            // what follows is called _jmp_mem by the sway compiler
            op::move_(0x16, RegId::SSP),
            op::sub(0x16, 0x16, RegId::IS),
            op::divi(0x16, 0x16, 4),
            op::jmp(0x16),
        ]
    };

    let real_num_of_instructions = get_instructions(0, blob_ids.len() as u16).len() as u16;

    let instruction_bytes: Vec<u8> =
        get_instructions(real_num_of_instructions, blob_ids.len() as u16)
            .into_iter()
            .collect();

    let blob_bytes: Vec<u8> = blob_ids.iter().flatten().copied().collect();

    [instruction_bytes, blob_bytes].concat()
}
