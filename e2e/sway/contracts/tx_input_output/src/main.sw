contract;

use std::{inputs::*, outputs::*};

configurable {
    ASSET_ID: AssetId = AssetId::zero(),
    OWNER: Address = Address::zero(),
}

abi TxContractTest {
    fn check_input(index: u64);
    fn check_output_is_change(index: u64);
}

impl TxContractTest for Contract {
    fn check_input(index: u64) {
        // Checks if coin and maybe returns owner
        if let Some(owner) = input_coin_owner(index) {
            require(owner == OWNER, "wrong owner");

            let asset_id = input_asset_id(index).unwrap();
            require(asset_id == ASSET_ID, "wrong asset id");
        } else {
            revert_with_log("input is not a coin");
        }
    }

    fn check_output_is_change(index: u64) {
        if let Some(Output::Change) = output_type(index) {
            let asset_to = output_asset_to(index).unwrap();
            require(asset_to == OWNER, "wrong change address");
        } else {
            revert_with_log("output is not change");
        }
    }
}
