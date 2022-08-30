library vec_lib;

use std::vec::Vec;

pub struct MyRawVec {
    ptr: u64,
    cap: u64

}

pub struct MyVec {
    raw: MyRawVec,
    len: u64
}


abi MyContract {
    fn real_vec(vec_1: Vec<u64>, vec_2: Vec<u64>);
    fn fake_vec(a_vector: MyVec);
    fn return_a_vec()->Vec<u64>;
}
