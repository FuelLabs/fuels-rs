contract;

use std::{bytes::Bytes, context::balance_of, context::msg_amount, message::send_message, token::*};

abi TestFuelCoin {
    fn mint_coins(mint_amount: u64);
    fn mint_to_addresses(mint_amount: u64, addresses: [Address; 3]);
    fn burn_coins(burn_amount: u64);
    fn force_transfer_coins(coins: u64, asset_id: ContractId, target: ContractId);
    fn transfer_coins_to_output(coins: u64, asset_id: ContractId, recipient: Address);
    fn get_balance(target: ContractId, asset_id: ContractId) -> u64;
    #[payable]
    fn get_msg_amount() -> u64;
    fn send_message(recipient: b256, coins: u64);
}

impl TestFuelCoin for Contract {
    fn mint_coins(mint_amount: u64) {
        mint(mint_amount);
    }
    fn mint_to_addresses(mint_amount: u64, addresses: [Address; 3]) {
        let mut counter = 0;
        while counter < 3 {
            mint_to_address(mint_amount, addresses[counter]);
            counter = counter + 1;
        }
    }

    fn burn_coins(burn_amount: u64) {
        burn(burn_amount);
    }

    fn force_transfer_coins(coins: u64, asset_id: ContractId, target: ContractId) {
        force_transfer_to_contract(coins, asset_id, target);
    }

    // ANCHOR: variable_outputs
    fn transfer_coins_to_output(coins: u64, asset_id: ContractId, recipient: Address) {
        transfer_to_address(coins, asset_id, recipient);
    }
    // ANCHOR_END: variable_outputs
    fn get_balance(target: ContractId, asset_id: ContractId) -> u64 {
        balance_of(target, asset_id)
    }

    #[payable]
    fn get_msg_amount() -> u64 {
        msg_amount()
    }

    fn send_message(recipient: b256, coins: u64) {
        let mut data = Bytes::new();
        data.push(1);
        data.push(2);
        data.push(3);

        send_message(recipient, data, coins);
    }
}
