contract;

abi MyContract {
    fn i_return_a_big_type() -> Vec<u8>;
    fn i_log_a_1000_el_array() ;
}

configurable {
    NUM_ELEMENTS: u64 = 0,
}

impl MyContract for Contract {
    fn i_log_a_1000_el_array() {
      let arr: [u8; 1000] = [0; 1000];
      log(arr);
    }

    fn i_return_a_big_type() -> Vec<u8> {
        let mut vec = Vec::new();
        let mut counter = 0;

        while counter < NUM_ELEMENTS {
            counter += 1;
            vec.push(0);
        }

        vec
    }
}
