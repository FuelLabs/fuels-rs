script;

use std::logging::log;

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

fn main() {
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
    let test_generic_struct = StructWithGeneric {
        field_1: test_struct,
        field_2: 64,
    };

    let test_generic_enum = EnumWithGeneric::VariantOne(l);
    let test_struct_nested = StructWithNestedGeneric {
        field_1: test_generic_struct,
        field_2: 64,
    };
    let test_deeply_nested_generic = StructDeeplyNestedGeneric {
        field_1: test_struct_nested,
        field_2: 64,
    };

    log(128);
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
    log(test_generic_enum);
    log(test_struct_nested);
    log(test_deeply_nested_generic);
}
