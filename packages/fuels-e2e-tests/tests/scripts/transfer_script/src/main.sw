script;

use std::token::transfer_to_address;

fn main(amount: u64, asset: AssetId, receiver: Address) -> () {
    transfer_to_address(receiver, asset, amount);
}
