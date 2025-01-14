script;

use std::string::String;

fn main(string: String) -> String {
    assert_eq(string, String::from_ascii_str("script-input"));

    String::from_ascii_str("script-return")
}
