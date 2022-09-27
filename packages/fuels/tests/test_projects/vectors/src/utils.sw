library utils;

use std::vec::Vec;
use std::logging::log;
use std::option::Option;
use std::assert::assert;

pub fn check_vec(arg: Vec<u32>) {
    assert(arg.len() == 3);

    let mut i = 0;
    while i < arg.len() {
        let option = arg.get(i);
        match option {
            Option::Some(val) => {
                assert(val == i);
            },
            _ => {
                assert(false);
            }
        };
        i += 1;
    }
}

pub fn log_vec<T>(vec: Vec<T>) {
    let mut i = 0;
    while i < vec.len() {
        let el = vec.get(i);

        match el {
            Option::Some(val) => {
                log(val)
            },
            _ => {
                assert(false);
            }
        };

        i += 1;
    }
}

pub fn expected_vec() -> Vec<u32> {
    let mut vec = ~Vec::new();
    vec.push(0);
    vec.push(1);
    vec.push(2);
    vec
}
