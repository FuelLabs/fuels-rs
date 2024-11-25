contract;

struct TestStruct {
    field_1: bool,
    field_2: u64,
}

#[allow(dead_code)]
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
    fn assert_primitive(a: u64, b: u64);

    fn assert_eq_primitive(a: u64, b: u64);
    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct);
    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum);

    fn assert_ne_primitive(a: u64, b: u64);
    fn assert_ne_struct(test_struct: TestStruct, test_struct2: TestStruct);
    fn assert_ne_enum(test_enum: TestEnum, test_enum2: TestEnum);
}

impl TestContract for Contract {
    fn assert_primitive(a: u64, b: u64) {
        assert(a == b);
    }

    fn assert_eq_primitive(a: u64, b: u64) {
        assert_eq(a, b);
    }
    fn assert_eq_struct(test_struct: TestStruct, test_struct2: TestStruct) {
        assert_eq(test_struct, test_struct2);
    }
    fn assert_eq_enum(test_enum: TestEnum, test_enum2: TestEnum) {
        assert_eq(test_enum, test_enum2);
    }

    fn assert_ne_primitive(a: u64, b: u64) {
        assert_ne(a, b);
    }
    fn assert_ne_struct(test_struct: TestStruct, test_struct2: TestStruct) {
        assert_ne(test_struct, test_struct2);
    }
    fn assert_ne_enum(test_enum: TestEnum, test_enum2: TestEnum) {
        assert_ne(test_enum, test_enum2);
    }
}
