predicate;

struct TestStruct {
    value: u64,
}

struct AnotherTestStruct {
    value: u64,
    number: u64,
}

fn main(test_struct: TestStruct, test_struct2: AnotherTestStruct) -> bool {
    test_struct.value == (test_struct2.value + test_struct2.number)
}
