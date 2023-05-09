contract;

use std::storage::storage_api::{read, write};
use std::constants::BASE_ASSET_ID;
use std::low_level_call::{call_with_function_selector, CallParams};
use std::bytes::Bytes;

abi MyContract {
    #[storage(write)]
    fn set_value(new_value: u64);
    #[storage(read)]
    fn get_value() -> u64;
    #[storage(write)]
    fn set_value_multiple_complex(a: MyStruct, b: str[4]);
    #[storage(read)]
    fn get_str_value() -> str[4];
    #[storage(read)]
    fn get_bool_value() -> bool;
    fn call_low_level_call(target: ContractId, function_selector: Bytes, calldata: Bytes, single_value_type_arg: bool);
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

storage {
    value: u64 = 0,
    value_b256: b256 = 0x0000000000000000000000000000000000000000000000000000000000000000,
    value_str: str[4] = "none",
    value_bool: bool = false,
}

pub struct MyStruct {
    a: bool,
    b: [u64; 3],
}

impl MyContract for Contract {
    #[storage(write)]
    fn set_value(value: u64) {
        write(COUNTER_KEY, 0, value);
    }

    #[storage(read)]
    fn get_value() -> u64 {
        read::<u64>(COUNTER_KEY, 0).unwrap_or(0)
    }

    #[storage(write)]
    fn set_value_multiple_complex(a: MyStruct, b: str[4]) {
        storage.value.write(a.b[1]);
        storage.value_str.write(b);
        storage.value_bool.write(a.a);
    }

    #[storage(read)]
    fn get_str_value() -> str[4] {
        storage.value_str.read()
    }

    #[storage(read)]
    fn get_bool_value() -> bool {
        storage.value_bool.read()
    }

    fn call_low_level_call(
        target: ContractId,
        function_selector: Bytes,
        calldata: Bytes,
        single_value_type_arg: bool,
    ) {
        let call_params = CallParams {
            coins: 0,
            asset_id: BASE_ASSET_ID,
            gas: 10_000,
        };

        call_with_function_selector(target, function_selector, calldata, single_value_type_arg, call_params);
    }
}
