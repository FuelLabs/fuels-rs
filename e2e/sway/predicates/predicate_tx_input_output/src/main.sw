predicate;

use std::{inputs::*, outputs::*};

configurable {
    ASSET_ID: AssetId = AssetId::zero(),
    OWNER: Address = Address::zero(),
}

fn main(input_index: u64, output_index: u64) -> bool {
    // Checks if coin and maybe returns owner
    let input_ok = if let Some(owner) = input_coin_owner(input_index) {
        let is_owner = owner == OWNER;

        let asset_id = input_asset_id(input_index).unwrap();

        is_owner && asset_id == ASSET_ID
    } else {
        false
    };

    let output_ok = if let Some(Output::Change) = output_type(output_index) {
        let asset_to = output_asset_to(output_index).unwrap();
        asset_to == OWNER
    } else {
        false
    };

    input_ok && output_ok
}
