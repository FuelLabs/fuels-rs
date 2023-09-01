contract;

use lib_contract::LibContract;

impl LibContract for Contract {
    fn increment(value: u64) -> u64 {
        value + 1
    }

    fn require() -> () {
        require(false, "require from contract");
    }
}
