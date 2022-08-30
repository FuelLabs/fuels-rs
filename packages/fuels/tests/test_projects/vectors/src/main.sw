contract;

use std::vec::Vec;
use std::option::Option;
use std::logging::log;
use std::assert::assert;
use vec_lib::{MyContract, MyVec};
use std::mem::addr_of;

fn check_vector(a_vec: Vec<u64>) {
        let mut i = 0;
        while i < a_vec.len() {
            assert(a_vec.get(i).unwrap() == ((i + 1) * 100));
            i += 1;
        }
}

fn print_a_vec(a_vec: Vec<u64>) {
        let mut i = 0;
        while i < a_vec.len() {
            log(a_vec.get(i).unwrap());
            i += 1;
        }
}

impl MyContract for Contract {
    fn real_vec(vec_1: Vec<u64>, vec_2: Vec<u64>){
        check_vector(vec_1);
        check_vector(vec_2);
    }

    fn return_a_vec()->Vec<u64>{
        let mut vec = ~Vec::new();

        vec.push(100);
        vec.push(200);
        vec.push(300);

        print_a_vec(vec);

        vec
    }

    fn fake_vec(a_vector: MyVec){

        assert(a_vector.raw.ptr == 10480 + 24);
        assert(a_vector.raw.cap == 4);
        assert(a_vector.len == 3);
        let base_addr = 10480;

        let ptr: u64 = std::mem::read(base_addr);
        assert(ptr == 10480 + 24);

        let cap: u64 = std::mem::read(base_addr + 8);
        assert(cap == 4);

        let len: u64 = std::mem::read(base_addr + 16);
        assert(len == 3);

        let el1: u64 = std::mem::read(base_addr + 24);
        assert(el1 == 100);

        let el2: u64 = std::mem::read(base_addr + 32);
        assert(el2 == 200);

        let el3: u64 = std::mem::read(base_addr + 40);
        assert(el3 == 300);

        //Token::U64(100),
        //Token::U64(200),
        //Token::U64(300),
        log(addr_of(a_vector));
    }
}
