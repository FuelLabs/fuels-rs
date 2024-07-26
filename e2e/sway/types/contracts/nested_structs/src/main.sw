contract;

pub struct SomeStruct {
    field: u32,
    field_2: bool,
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

#[allow(dead_code)]
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
                field: 12345u32,
                field_2: true,
            },
        }
    }
    fn check_struct_integrity(arg: AllStruct) -> bool {
        arg.some_struct.field == 12345u32 && arg.some_struct.field_2 == true
    }

    fn nested_struct_with_reserved_keyword_substring(call_data: CallData) -> CallData {
        call_data
    }
}
