contract;

use std::string::String;

abi MyContract {
     fn return_dynamic_string() -> String;
}

impl MyContract for Contract {
     fn return_dynamic_string() -> String {
       String::from_ascii_str("hello world")
     }
}
