contract;
use std::vec::Vec;
use std::mem::addr_of;
use std::logging::log;
use std::option::Option;
use std::assert::assert;

abi MyContract {
    fn test_function(arg: Vec<u32>);
}

struct SomeStruct{
    a: u32
}


impl MyContract for Contract {
    fn test_function(arg: Vec<u32>){
        assert(arg.len() == 3);

        let mut i = 0;
        while i < arg.len() {
            let value = arg.get(i);
            match value {
                Option::Some(val) => {
                    assert(val == i);
                },
                _ => {
                    //assert(false);
                }
            };
            i += 1;
        }
    }
}
