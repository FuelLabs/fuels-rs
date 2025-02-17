contract;

use lib_contract_abi::LibContract;

impl LibContract for Contract {
    fn increment(value: u64) -> u64 {
        value + 1
    }

    fn require() -> () {
        require(false, __to_str_array("require from contract"));
    }
}
