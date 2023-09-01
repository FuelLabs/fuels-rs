contract;

abi MyContract {
    fn get_current_height() -> u32;
    fn calling_this_will_produce_a_block();
}

impl MyContract for Contract {
    fn get_current_height() -> u32 {
        std::block::height()
    }

    fn calling_this_will_produce_a_block() {}
}
