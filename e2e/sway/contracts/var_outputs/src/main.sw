contract;

abi MyContract {
    fn mint(coins: u64, recipient: Identity);
}

impl MyContract for Contract {
    fn mint(coins: u64, recipient: Identity) {
        let mut counter = 0;
        while counter < coins {
            counter += 1;
            std::asset::mint_to(recipient, std::constants::ZERO_B256, 1);
        }
    }
}
