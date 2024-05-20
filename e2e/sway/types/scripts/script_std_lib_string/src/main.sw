script;

use std::string::String;

fn main(string: String) {
    assert_eq(string, String::from_ascii_str("Hello World"));
}
