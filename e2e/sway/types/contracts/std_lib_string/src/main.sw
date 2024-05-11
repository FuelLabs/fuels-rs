contract;

use std::string::String;
use std::assert::assert_eq;
use std::bytes::Bytes;

abi MyContract {
    fn return_dynamic_string() -> String;
    fn accepts_dynamic_string(s: String);
    fn echoes_dynamic_string(s: String) -> String;
}

impl MyContract for Contract {
    fn return_dynamic_string() -> String {
        String::from_ascii_str("Hello World")
    }

    fn accepts_dynamic_string(string: String) {
        assert_eq(string, String::from_ascii_str("Hello World"));
    }

    fn echoes_dynamic_string(string: String) -> String {
        string
    }
}
