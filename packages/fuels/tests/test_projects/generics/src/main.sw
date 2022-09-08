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
    foo: AnotherOne<AGenericEn<T>>,
    boo: K,
}

enum AGenericEn<T> {
    one: u64,
    two: T,
}

abi MyContract {
    fn identity(arg: MyStruct<u64, AnotherOne<BraveOne>>) -> MyStruct<u64, AnotherOne<BraveOne>>;
    //fn enum_using(arg: AGenericEn<MyStruct<u64, AnotherOne<BraveOne>>>) -> u64;
}

impl MyContract for Contract {
    fn identity(arg: MyStruct<u32, AnotherOne<BraveOne>>) -> MyStruct<u32, AnotherOne<BraveOne>> {
        arg
    }
    //fn enum_using(arg: AGenericEn<MyStruct<u64, AnotherOne<BraveOne>>>) -> u64 {
    //    64
    //}
}
