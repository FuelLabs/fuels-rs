script;

use std::{logging::log, string::String};

#[allow(dead_code)]
enum EnumWithGeneric<D> {
    VariantOne: D,
    VariantTwo: (),
}

fn main() {
    // String slice
    log("fuel");

    // String
    log(String::from_ascii_str("fuel"));

    // Bytes
    log(String::from_ascii_str("fuel").as_bytes());

    // RawSlice
    log(String::from_ascii_str("fuel").as_raw_slice());

    // Vector
    let mut v = Vec::new();
    v.push(1u16);
    v.push(2u16);
    v.push(3u16);

    let some_enum = EnumWithGeneric::VariantOne(v);
    let other_enum = EnumWithGeneric::VariantTwo;

    let mut v1 = Vec::new();
    v1.push(some_enum);
    v1.push(other_enum);
    v1.push(some_enum);

    let mut v2 = Vec::new();
    v2.push(v1);
    v2.push(v1);

    let mut v3 = Vec::new();
    v3.push(v2);

    log(v3);
}
