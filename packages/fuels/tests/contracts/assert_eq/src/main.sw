contract;

// TODO: Remove when bug is fixed.
use core::ops::*;
fn assert_eq<T>(v1: T, v2: T)
where
    T: Eq
{
    let FAILED_ASSERT_EQ_SIGNAL = 0xffff_ffff_ffff_0003;
    if (v1 != v2) {
        log(v1);
        log(v2);
        revert(FAILED_ASSERT_EQ_SIGNAL);
    }
}
// issue link: https://github.com/FuelLabs/fuels-rs/issues/829
struct TestStruct {
    field_1: bool,
    field_2: u64,
}

enum TestEnum {
    VariantOne: (),
    VariantTwo: (),
}

impl Eq for TestStruct {
    fn eq(self, other: Self) -> bool {
        self.field_1 == other.field_1 && self.field_2 == other.field_2
    }
}

impl Eq for TestEnum {
    fn eq(self, other: Self) -> bool {
        match (self, other) {
            (TestEnum::VariantOne, TestEnum::VariantOne) => true,
            (TestEnum::VariantTwo, TestEnum::VariantTwo) => true,
            _ => false,
        }
    }
}

abi TestContract {
    fn assert_eq_primitive(a: u64, b: u64);
    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct);
    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum);
}

impl TestContract for Contract {
    fn assert_eq_primitive(a: u64, b: u64) {
        assert_eq(a, b);
    }

    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct) {
        assert_eq(test_struct, test_struct2);
    }

    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum) {
        assert_eq(test_enum, test_enum2);
    }
}
