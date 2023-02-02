contract;

use std::storage::get;

storage {
    x: u64 = 64,
    y: b256 = 0x0101010101010101010101010101010101010101010101010101010101010101,
}

abi MyContract {
    #[storage(read)]
    fn get_value_b256(key: b256) -> b256;
    #[storage(read)]
    fn get_value_u64(key: b256) -> u64;
}

impl MyContract for Contract {
    #[storage(read)]
    fn get_value_b256(key: b256) -> b256 {
        get::<b256>(key).unwrap()
    }

    #[storage(read)]
    fn get_value_u64(key: b256) -> u64 {
        get::<u64>(key).unwrap()
    }
}
