script;

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
    AssertEqPrimitive: (u64, u64),
    AssertEqStruct: (TestStruct, TestStruct),
    AssertEqEnum: (TestEnum, TestEnum),
}

fn main(match_enum: MatchEnum) {
    if let MatchEnum::AssertEqPrimitive((a, b)) = match_enum {
        log(a);
        log(b);
        revert(ASSERT_EQ_SIGNAL);
    } else if let MatchEnum::AssertEqStruct((test_struct, test_struct2)) = match_enum
    {
        log(test_struct);
        log(test_struct2);
        revert(ASSERT_EQ_SIGNAL);
    } else if let MatchEnum::AssertEqEnum((test_enum, test_enum2)) = match_enum
    {
        log(test_enum);
        log(test_enum2);
        revert(ASSERT_EQ_SIGNAL);
    }
}
