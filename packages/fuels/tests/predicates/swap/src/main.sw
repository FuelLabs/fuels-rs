predicate;

use std::{
    constants::BASE_ASSET_ID,
    outputs::{
        Output,
        output_amount,
        output_asset_id,
        output_asset_to,
    },
};

fn main() -> bool {
    let receiver = Address::from(0x09c0b2d1a486c439a87bcba6b46a7a1a23f3897cc83a94521a96da5c23bc58db);
    let ask_amount = 100;

    let output_index = 0;
    let to = Address::from(output_asset_to(output_index).unwrap());
    let asset_id = output_asset_id(output_index).unwrap();
    let amount = output_amount(output_index);
    (to == receiver) && (amount == ask_amount) && (asset_id == BASE_ASSET_ID)
}
