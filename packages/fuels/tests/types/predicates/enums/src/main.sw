predicate;

enum TestEnum {
    A: u64,
}

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
