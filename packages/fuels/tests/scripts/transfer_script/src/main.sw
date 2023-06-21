script;

use std::token::transfer_to_address;

fn main(amount: u64, asset: ContractId, receiver: Address) -> () {
    transfer_to_address(amount, asset, receiver);
}
