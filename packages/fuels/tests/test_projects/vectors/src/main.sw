contract;
use std::vec::Vec;

abi MyContract {
    fn test_function(arg: Vec<u32>);
}

impl MyContract for Contract {
    fn test_function(arg: Vec<u32>){
    }
}
