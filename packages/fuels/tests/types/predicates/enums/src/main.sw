predicate;

#[allow(dead_code)]
enum TestEnum {
    A: u64,
}

#[allow(dead_code)]
enum AnotherTestEnum {
    A: u64,
    B: u64,
}

fn main(test_enum: TestEnum, test_enum2: AnotherTestEnum) -> bool {
    if let TestEnum::A(a) = test_enum {
        if let AnotherTestEnum::B(b) = test_enum2 {
            return a == b;
        }
    }

    false
}
