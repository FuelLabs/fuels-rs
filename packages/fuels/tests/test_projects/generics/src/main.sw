contract;

use std::assert::assert;
use core::ops::Eq;

struct SimpleGeneric<T> {
    single_generic_param: T,
}

struct PassTheGenericOn<T> {
    one: SimpleGeneric<T>
}

struct MegaExample<T,U> {
    a: ([U;2], T),
}

abi MyContract {
    fn complex_test(arg1: MegaExample<PassTheGenericOn<u64>, SimpleGeneric<str[2]>>);

}

impl MyContract for Contract {
    fn complex_test(arg1: MegaExample<PassTheGenericOn<u64>, SimpleGeneric<str[2]>>){
    }
}
