contract;

abi MyContract {
    fn i_return_a_1k_el_array() -> [u8; 1000];
    fn i_log_a_1k_el_array();
}

impl MyContract for Contract {
    fn i_log_a_1k_el_array() {
        let arr: [u8; 1000] = [0; 1000];
        log(arr);
    }

    fn i_return_a_1k_el_array() -> [u8; 1000] {
        [0; 1000]
    }
}
