library utils;

use std::vec::Vec;
use std::logging::log;
use std::option::Option;
use std::assert::assert;

// ANCHOR: sway_log_vec_helper
pub fn log_vec<T>(vec: Vec<T>) {
    let mut i = 0;
    while i < vec.len() {
        log(vec.get(i).unwrap());
        i += 1;
    }
}
// ANCHOR_END: sway_log_vec_helper
pub fn vec_from(vals: [u32; 3]) -> Vec<u32> {
    let mut vec = ~Vec::new();
    vec.push(vals[0]);
    vec.push(vals[1]);
    vec.push(vals[2]);
    vec
}
