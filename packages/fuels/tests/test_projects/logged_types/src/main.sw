contract;

use std::*;
use core::*;
use std::storage::*;
use std::context::msg_amount;

struct TestStruct {
    field_1: bool,
    field_2: b256,
    field_3: u64,
}

enum TestEnum {
    VariantOne: (),
    VariantTwo: (),
}

abi TestContract {
    fn produce_logs() -> ();
}

impl TestContract for Contract {
    fn produce_logs() -> () {
        let r = 42;
        let k: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        let a: str[4] = "Fuel";
        let b: [u8; 3] = [1u8, 2u8, 3u8];
        let test_struct = TestStruct {
            field_1: true,
            field_2: k,
            field_3: 11,
        };

        let test_enum = TestEnum::VariantTwo;
        __log(r);
        __log(k);
        __log(41);
        __log(42u32);
        __log(42u16);
        __log(42u8);
        __log(a);
        __log(b);
        __log(test_struct);
        __log(test_enum);
    }
}