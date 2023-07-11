contract;

use std::string::String;

abi MyContract {
    fn return_dynamic_string() -> String;
    // The content of the `String` passed as argument is not tested because it doesn't seem like
    // it is stable (https://github com/FuelLabs/sway/issues/4788). We just check that the
    // function can be called.
    fn accepts_dynamic_string(s: String) -> bool;
}

impl MyContract for Contract {
    fn return_dynamic_string() -> String {
        String::from_ascii_str("hello world")
    }

    fn accepts_dynamic_string(s: String) -> bool {
        assert(s.capacity() == 0); // this is true even when `s` is not empty
        assert(s.as_bytes().len() == 1_000_000); // this is true as well and contr
        assert(s.as_bytes().capacity() == 0); // this is also true and seems not coherent
        true
    }
}
