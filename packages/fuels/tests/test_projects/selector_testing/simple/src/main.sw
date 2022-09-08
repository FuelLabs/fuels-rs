contract;

struct ASimpleGeneric<T> {
	a: u64,
	b: T
}

abi MyContract {
    fn test_function(arg1: ASimpleGeneric<u32>);
}

impl MyContract for Contract {
    fn test_function(arg1: ASimpleGeneric<u32>){
    }
}
