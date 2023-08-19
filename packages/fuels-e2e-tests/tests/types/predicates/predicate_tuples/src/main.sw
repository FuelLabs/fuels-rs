predicate;

struct TestStruct {
    value: u32,
}

#[allow(dead_code)]
enum TestEnum {
    Value: u64,
}

fn main(input_tuple: (u64, TestStruct, TestEnum), number: u64) -> bool {
    let (u64_number, test_struct, test_enum) = input_tuple;

    if let TestEnum::Value(enum_value) = test_enum {
        return u64_number == 16 && test_struct.value == 32u32 && enum_value == 64 && number == 128;
    }

    false
}
