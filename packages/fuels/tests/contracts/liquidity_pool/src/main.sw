contract;

use std::{
    call_frames::{
        contract_id,
        msg_asset_id,
    },
    context::msg_amount,
    token::{
        mint_to_address,
        transfer_to_address,
    },
};
use std::constants::ZERO_B256;

abi LiquidityPool {
    #[payable]
    fn deposit(recipient: Address);
    #[payable]
    fn withdraw(recipient: Address);
}

const BASE_TOKEN: b256 = 0x9ae5b658754e096e4d681c548daf46354495a437cc61492599e33fc64dcdc30c;

impl LiquidityPool for Contract {
    #[payable]
    fn deposit(recipient: Address) {
        assert(BASE_TOKEN == msg_asset_id());
        assert(0 < msg_amount());

        // Mint two times the amount.
        let amount_to_mint = msg_amount() * 2;

        // Mint some LP token based upon the amount of the base token.
        mint_to_address(recipient, ZERO_B256, amount_to_mint);
    }

    #[payable]
    fn withdraw(recipient: Address) {
        assert(0 < msg_amount());

        // Amount to withdraw.
        let amount_to_transfer = msg_amount() / 2;

        // Transfer base token to recipient.
        transfer_to_address(recipient, BASE_TOKEN, amount_to_transfer);
    }
}
