contract;

use std::{
    asset::*,
    bytes::Bytes,
    constants::ZERO_B256,
    context::balance_of,
    context::msg_amount,
    message::send_message,
};

abi TestFuelCoin {
    fn mint_coins(mint_amount: u64);
    fn mint_to_addresses(mint_amount: u64, addresses: [Address; 3]);
    fn burn_coins(burn_amount: u64);
    fn force_transfer_coins(coins: u64, asset_id: AssetId, target: ContractId);
    fn transfer_coins_to_output(coins: u64, asset_id: AssetId, recipient: Address);
    fn get_balance(target: ContractId, asset_id: AssetId) -> u64;
    #[payable]
    fn get_msg_amount() -> u64;
    fn send_message(recipient: b256, coins: u64);
}

impl TestFuelCoin for Contract {
    fn mint_coins(mint_amount: u64) {
        mint(ZERO_B256, mint_amount);
    }
    fn mint_to_addresses(mint_amount: u64, addresses: [Address; 3]) {
        let mut counter = 0;
        while counter < 3 {
            mint_to_address(addresses[counter], ZERO_B256, mint_amount);
            counter = counter + 1;
        }
    }

    fn burn_coins(burn_amount: u64) {
        burn(ZERO_B256, burn_amount);
    }

    fn force_transfer_coins(coins: u64, asset_id: AssetId, target: ContractId) {
        force_transfer_to_contract(target, asset_id, coins);
    }

    // ANCHOR: variable_outputs
    fn transfer_coins_to_output(coins: u64, asset_id: AssetId, recipient: Address) {
        transfer_to_address(recipient, asset_id, coins);
    }
    // ANCHOR_END: variable_outputs
    fn get_balance(target: ContractId, asset_id: AssetId) -> u64 {
        balance_of(target, asset_id)
    }

    #[payable]
    fn get_msg_amount() -> u64 {
        msg_amount()
    }

    fn send_message(recipient: b256, coins: u64) {
        let mut data = Bytes::new();
        data.push(1u8);
        data.push(2u8);
        data.push(3u8);

        send_message(recipient, data, coins);
    }
}
