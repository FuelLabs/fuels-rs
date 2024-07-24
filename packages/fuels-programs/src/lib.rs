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
