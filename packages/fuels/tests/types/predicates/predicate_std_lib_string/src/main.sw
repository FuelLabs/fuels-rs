predicate;

use std::string::String;

fn validate_string(string: String) -> bool {
    string == String::from_ascii_str("Hello World")
}

fn main(_arg_0: u64, _arg_1: u64, string: String) -> bool {
    validate_string(string)
}
