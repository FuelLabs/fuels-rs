contract;

use std::logging::log;
use contract_logs::ContractLogs;

#[allow(dead_code)]
struct TestStruct {
    field_1: bool,
    field_2: b256,
    field_3: u64,
}

#[allow(dead_code)]
enum TestEnum {
    VariantOne: (),
    VariantTwo: (),
}

#[allow(dead_code)]
struct StructWithGeneric<D> {
    field_1: D,
    field_2: u64,
}

#[allow(dead_code)]
enum EnumWithGeneric<D> {
    VariantOne: D,
    VariantTwo: (),
}

#[allow(dead_code)]
struct StructWithNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

#[allow(dead_code)]
struct StructDeeplyNestedGeneric<D> {
    field_1: D,
    field_2: u64,
}

impl ContractLogs for Contract {
    fn produce_logs_values() {
        log(64u64);
        log(32u32);
        log(16u16);
        log(8u8);
    }

    // ANCHOR: produce_logs
    fn produce_logs_variables() {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        let e: str[4] = __to_str_array("Fuel");
        let l: [u8; 3] = [1u8, 2u8, 3u8];

        log(f);
        log(u);
        log(e);
        log(l);
    }
    // ANCHOR_END: produce_logs
    fn produce_logs_custom_types() {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;

        let test_struct = TestStruct {
            field_1: true,
            field_2: u,
            field_3: f,
        };
        let test_enum = TestEnum::VariantTwo;

        log(test_struct);
        log(test_enum);
    }

    fn produce_logs_generic_types() {
        let l: [u8; 3] = [1u8, 2u8, 3u8];

        let test_struct = StructWithGeneric {
            field_1: l,
            field_2: 64,
        };
        let test_enum = EnumWithGeneric::VariantOne(l);
        let test_struct_nested = StructWithNestedGeneric {
            field_1: test_struct,
            field_2: 64,
        };
        let test_deeply_nested_generic = StructDeeplyNestedGeneric {
            field_1: test_struct_nested,
            field_2: 64,
        };

        log(test_struct);
        log(test_enum);
        log(test_struct_nested);
        log(test_deeply_nested_generic);
    }

    fn produce_multiple_logs() {
        let f: u64 = 64;
        let u: b256 = 0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a;
        let e: str[4] = __to_str_array("Fuel");
        let l: [u8; 3] = [1u8, 2u8, 3u8];
        let test_struct = TestStruct {
            field_1: true,
            field_2: u,
            field_3: f,
        };
        let test_enum = TestEnum::VariantTwo;
        let test_generic_struct = StructWithGeneric {
            field_1: test_struct,
            field_2: 64,
        };

        log(64);
        log(32u32);
        log(16u16);
        log(8u8);
        log(f);
        log(u);
        log(e);
        log(l);
        log(test_struct);
        log(test_enum);
        log(test_generic_struct);
    }

    fn produce_bad_logs() {
        // produce a custom log with log id 128
        // this log id will not be present in abi JSON
        asm(r1: 0, r2: 128, r3: 0, r4: 0) {
            log r1 r2 r3 r4;
        }

        log(123);
    }

    fn produce_string_slice_log() {
        log("string_slice");
    }
}
