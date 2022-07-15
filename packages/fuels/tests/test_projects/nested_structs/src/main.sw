contract;

use std::contract_id::ContractId;

pub struct SomeStruct {
    par1: u32,
}

pub struct AllStruct {
    some_struct: SomeStruct,
}

pub struct CallData {
    memory_address: MemoryAddress,
    num_coins_to_forward: u64,
    asset_id_of_coins_to_forward: ContractId,
    amount_of_gas_to_forward: u64,
}

struct MemoryAddress {
    contract_id: ContractId,
    function_selector: u64,
    function_data: u64,
}

abi MyContract {
    fn get_struct() -> AllStruct;
    fn check_struct_integrity(arg: AllStruct) -> bool;
    fn nested_struct_with_reserved_keyword_substring(call_data: CallData) -> CallData;
}

impl MyContract for Contract {
    fn get_struct() -> AllStruct {
        AllStruct {
            some_struct: SomeStruct {
                par1: 12345u32,
            },
        }
    }
    fn check_struct_integrity(arg: AllStruct) -> bool {
        arg.some_struct.par1 == 12345u32
    }

    fn nested_struct_with_reserved_keyword_substring(call_data: CallData) -> CallData {
        call_data
    }
}
