script;

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

enum MatchEnum {
    AssertPrimitive: (u64, u64),
    AssertEqPrimitive: (u64, u64),
    AssertEqStruct: (TestStruct, TestStruct),
    AssertEqEnum: (TestEnum, TestEnum),
}

fn main(match_enum: MatchEnum) {
    if let MatchEnum::AssertPrimitive((a, b)) = match_enum {
        assert(a == b);
    } else if let MatchEnum::AssertEqPrimitive((a, b)) = match_enum {
        assert_eq(a, b);
    } else if let MatchEnum::AssertEqStruct((test_struct, test_struct2)) = match_enum
    {
        assert_eq(test_struct, test_struct2);
    } else if let MatchEnum::AssertEqEnum((test_enum, test_enum2)) = match_enum
    {
        assert_eq(test_enum, test_enum2);
    }
}
