predicate;

use std::string::String;

fn main(_arg_0: u64, _arg_1: u64, string: String) -> bool {
    string == String::from_ascii_str("Hello World")
}
