library utils;

use std::vec::Vec;
use std::logging::log;
use std::option::Option;
use std::assert::assert;

pub fn log_vec<T>(vec: Vec<T>) {
    let mut i = 0;
    while i < vec.len() {
        let el = vec.get(i);

        match el {
            Option::Some(val) => log(val),
            _ => assert(false),
        };

        i += 1;
    }
}

pub fn vec_from(vals: [u32; 3]) -> Vec<u32> {
    let mut vec = ~Vec::new();
    vec.push(vals[0]);
    vec.push(vals[1]);
    vec.push(vals[2]);
    vec
}
