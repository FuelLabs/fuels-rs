contract;

abi MyContract {
    fn i_return_a_big_type() -> Vec<u8>;
}

configurable {
    NUM_TOKENS: u64 = 0,
}

impl MyContract for Contract {
    fn i_return_a_big_type() -> Vec<u8> {
        let mut vec = Vec::new();
        let mut counter = 0;

        while counter < NUM_TOKENS {
            counter += 1;
            vec.push(0);
        }

        vec
    }
}
