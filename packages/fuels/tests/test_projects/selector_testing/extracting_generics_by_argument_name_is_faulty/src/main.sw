contract;

struct InnerGeneric<Z>{
	c: Z
}

struct BaseGeneric<T, U>{
	a: T,
	b: InnerGeneric<U>
}

abi MyContract {
    fn test_function(arg1: BaseGeneric<u64, u32>);
}

impl MyContract for Contract {
    fn test_function(arg1: BaseGeneric<u64, u32>){
    }
}
