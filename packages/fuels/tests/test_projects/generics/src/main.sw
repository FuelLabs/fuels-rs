contract;

struct BraveOne {
    one: u8,
    two: u64,
}

struct AnotherOne<T> {
    rodrigo: u64,
    john: str[15],
    juicy: T,
}

struct MyStruct<T, K> {
    foo: T,
    boo: K,
}

abi MyContract {
    fn identity(arg: MyStruct<u64, AnotherOne<BraveOne>>) -> MyStruct<u64, AnotherOne<BraveOne>>;
}

impl MyContract for Contract {
    fn identity(arg: MyStruct<u64, AnotherOne<BraveOne>>) -> MyStruct<u64, AnotherOne<BraveOne>> {
        arg
    }
}
