script;

struct TestStruct {
    field_1: bool,
    field_2: u64,
}

#[allow(dead_code)]
enum TestEnum {
    VariantOne: (),
    VariantTwo: (),
}

impl PartialEq for TestStruct {
    fn eq(self, other: Self) -> bool {
        self.field_1 == other.field_1 && self.field_2 == other.field_2
    }
}
impl Eq for TestStruct {}

impl PartialEq for TestEnum {
    fn eq(self, other: Self) -> bool {
        match (self, other) {
            (TestEnum::VariantOne, TestEnum::VariantOne) => true,
            (TestEnum::VariantTwo, TestEnum::VariantTwo) => true,
            _ => false,
        }
    }
}
impl Eq for TestEnum {}

#[allow(dead_code)]
enum MatchEnum {
    AssertPrimitive: (u64, u64),
    AssertEqPrimitive: (u64, u64),
    AssertEqStruct: (TestStruct, TestStruct),
    AssertEqEnum: (TestEnum, TestEnum),
    AssertNePrimitive: (u64, u64),
    AssertNeStruct: (TestStruct, TestStruct),
    AssertNeEnum: (TestEnum, TestEnum),
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
    } else if let MatchEnum::AssertNePrimitive((a, b)) = match_enum {
        assert_ne(a, b);
    } else if let MatchEnum::AssertNeStruct((test_struct, test_struct2)) = match_enum
    {
        assert_ne(test_struct, test_struct2);
    } else if let MatchEnum::AssertNeEnum((test_enum, test_enum2)) = match_enum
    {
        assert_ne(test_enum, test_enum2);
    }
}
