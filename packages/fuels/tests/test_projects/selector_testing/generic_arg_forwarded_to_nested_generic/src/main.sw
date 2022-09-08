contract;

struct InnerGeneric<U>{
	a: u64,
	b: U
}

struct BaseGeneric<T>{
	u: InnerGeneric<T>
}

abi MyContract {
    fn test_function(arg1: BaseGeneric<u32>);
}

impl MyContract for Contract {
    fn test_function(arg1: BaseGeneric<u32>){
    }
}
