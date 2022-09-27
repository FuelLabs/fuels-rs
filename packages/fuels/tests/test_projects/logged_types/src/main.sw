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
    fn produce_logs_values() -> ();
    fn produce_logs_variables() -> ();
    fn produce_logs_custom_types() -> ();
    fn produce_multiple_logs() -> ();
}

impl TestContract for Contract {
    fn produce_logs_values() {
        __log(64);
        __log(32u32);
        __log(16u16);
        __log(8u8);
    }

    // ANCHOR: produce_logs
    fn produce_logs_variables() -> () {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        let e: str[4] = "Fuel";
        let l: [u8; 3] = [1u8, 2u8, 3u8];

        __log(f);
        __log(u);
        __log(e);
        __log(l);
    }
    // ANCHOR_END: produce_logs

    fn produce_logs_custom_types() -> () {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        
        let test_struct = TestStruct {
            field_1: true,
            field_2: u,
            field_3: f,
        };
        let test_enum = TestEnum::VariantTwo;

        __log(test_struct);
        __log(test_enum);
    }

    fn produce_multiple_logs() -> () {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        let e: str[4] = "Fuel";
        let l: [u8; 3] = [1u8, 2u8, 3u8];
        let test_struct = TestStruct {
            field_1: true,
            field_2: u,
            field_3: f,
        };
        let test_enum = TestEnum::VariantTwo;

        __log(64);
        __log(32u32);
        __log(16u16);
        __log(8u8);
        __log(f);
        __log(u);
        __log(e);
        __log(l);
        __log(test_struct);
        __log(test_enum);
    }
}
