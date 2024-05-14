script;

use std::asset::transfer;

fn main(amount: u64, asset: AssetId, receiver: Identity) -> () {
    transfer(receiver, asset, amount);
}
