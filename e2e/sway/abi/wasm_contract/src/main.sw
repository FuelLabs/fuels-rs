contract;

enum SomeEnum<T> {
    V1: (),
    V2: T,
}

#[allow(dead_code)]
struct SomeStruct {
    a: u32,
    b: bool,
}

abi TestContract {
    fn test_function(arg: SomeEnum<SomeStruct>);
}

impl TestContract for Contract {
    fn test_function(_arg: SomeEnum<SomeStruct>) {}
}
