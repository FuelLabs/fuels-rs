contract;

struct SomeGenericStruct<T,U> {
	a: u64,
	b: T,
	c: U,
	d: T
}


abi MyContract {
    fn test_function(arg1: SomeGenericStruct<u8, u32>);
}

impl MyContract for Contract {
    fn test_function(arg1: SomeGenericStruct<u8, u32>) {
    }
}
