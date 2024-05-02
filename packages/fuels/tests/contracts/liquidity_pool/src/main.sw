contract;

use std::{
    asset::{
        mint_to,
        transfer,
    },
    call_frames::{
        msg_asset_id,
    },
    constants::ZERO_B256,
    context::msg_amount,
};

abi LiquidityPool {
    #[payable]
    fn deposit(recipient: Identity);
    #[payable]
    fn withdraw(recipient: Identity);
}

const BASE_TOKEN: AssetId = AssetId::from(0x9ae5b658754e096e4d681c548daf46354495a437cc61492599e33fc64dcdc30c);

impl LiquidityPool for Contract {
    #[payable]
    fn deposit(recipient: Identity) {
        assert(BASE_TOKEN == msg_asset_id());
        assert(0 < msg_amount());

        // Mint two times the amount.
        let amount_to_mint = msg_amount() * 2;

        // Mint some LP token based upon the amount of the base token.
        mint_to(recipient, ZERO_B256, amount_to_mint);
    }

    #[payable]
    fn withdraw(recipient: Identity) {
        assert(0 < msg_amount());

        // Amount to withdraw.
        let amount_to_transfer = msg_amount() / 2;

        // Transfer base token to recipient.
        transfer(recipient, BASE_TOKEN, amount_to_transfer);
    }
}
