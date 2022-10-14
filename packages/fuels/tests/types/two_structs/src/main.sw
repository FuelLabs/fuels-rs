contract;

pub struct StructOne {
    foo: u64,
}

pub struct StructTwo {
    bar: u64,
}

abi MyTest {
    fn something(input: StructOne) -> u64;
    fn something_else(input: StructTwo) -> u64;
}

impl MyTest for Contract {
    fn something(input: StructOne) -> u64 {
        let v = input.foo;
        v + 1
    }
    fn something_else(input: StructTwo) -> u64 {
        let v = input.bar;
        v - 1
    }
}
