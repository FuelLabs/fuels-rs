script;

use std::{inputs::*, outputs::*};

configurable {
    ASSET_ID: AssetId = AssetId::zero(),
    OWNER: Address = Address::zero(),
}

fn main(input_index: u64, output_index: u64) {
    // Checks if coin and maybe returns owner
    if let Some(owner) = input_coin_owner(input_index) {
        require(owner == OWNER, "wrong owner");

        let asset_id = input_asset_id(input_index).unwrap();
        require(asset_id == ASSET_ID, "wrong asset id");
    } else {
        revert_with_log("input is not a coin");
    }

    if let Some(Output::Change) = output_type(output_index) {
        let asset_to = output_asset_to(output_index).unwrap();
        require(asset_to == OWNER, "wrong change address");
    } else {
        revert_with_log("output is not change");
    }
}
