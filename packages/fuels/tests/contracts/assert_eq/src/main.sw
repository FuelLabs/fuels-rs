contract;

use std::assert::assert_eq;
use std::logging::log;

const ASSERT_EQ_SIGNAL = 0xffff_ffff_ffff_0003;

struct TestStruct {
    field_1: bool,
    field_2: u64,
}

enum TestEnum {
    VariantOne: (),
    VariantTwo: (),
}

enum MatchEnum {
    AssertEqPrimitive: (),
    AssertEqStruct: (),
    AssertEqEnum: (),
}

abi TestContract {
    fn assert_eq_primitive(a: u64, b: u64);
    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct);
    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum);
}

impl TestContract for Contract {
    fn assert_eq_primitive(a: u64, b: u64) {
        log(a);
        log(b);
        revert(ASSERT_EQ_SIGNAL);
    }

    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct) {
        log(test_struct);
        log(test_struct2);
        revert(ASSERT_EQ_SIGNAL);
    }

    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum) {
        log(test_enum);
        log(test_enum2);
        revert(ASSERT_EQ_SIGNAL);
    }
}
