contract;

use std::{
    address::Address,
    assert::assert,
    context::call_frames::{contract_id, msg_asset_id},
    context::msg_amount,
    contract_id::ContractId,
    token::{mint_to_address, transfer_to_address}
};

abi LiquidityPool {
    fn deposit(recipient: Address);
    fn withdraw(recipient: Address);
}

const BASE_TOKEN: b256 = 0x9ae5b658754e096e4d681c548daf46354495a437cc61492599e33fc64dcdc30c;

impl LiquidityPool for Contract {
    fn deposit(recipient: Address) {
        assert(~ContractId::from(BASE_TOKEN) == msg_asset_id());
        assert(0 < msg_amount());

        // Mint two times the amount.
        let amount_to_mint = msg_amount() * 2;

        // Mint some LP token based upon the amount of the base token.
        mint_to_address(amount_to_mint, recipient);
    }

    fn withdraw(recipient: Address) {
       assert(contract_id() == msg_asset_id());
       assert(0 < msg_amount());

        // Amount to withdraw.
        let amount_to_transfer = msg_amount() / 2;

        // Transfer base token to recipient.
        transfer_to_address(amount_to_transfer, ~ContractId::from(BASE_TOKEN), recipient);
    }
}
