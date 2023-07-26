contract;

use std::string::String;
use std::assert::assert_eq;
use std::bytes::Bytes;

abi MyContract {
    fn return_dynamic_string() -> String;
    fn accepts_dynamic_string(s: String);
}

impl MyContract for Contract {
    fn return_dynamic_string() -> String {
        String::from_ascii_str("hello world")
    }

    fn accepts_dynamic_string(s: String) {
        let inner = String::from_ascii_str("hello wol");
        log(inner.bytes.len());
        log(s.bytes.len());
        assert_eq(inner.as_bytes(), s.as_bytes());
    }
}
