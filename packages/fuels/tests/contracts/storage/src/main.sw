contract;

use std::storage::storage_api::read;

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
        read::<b256>(key, 0).unwrap()
    }

    #[storage(read)]
    fn get_value_u64(key: b256) -> u64 {
        read::<u64>(key, 0).unwrap()
    }
}
