contract;

use std::address::Address;

struct User {
    address: Address,
    weight: u64
}

abi MyContract {
    fn wrapped_address(user: User) -> User;
    fn unwrapped_address(addr: Address) -> Address;
}

impl MyContract for Contract {
    fn wrapped_address(user: User) -> User {
        user
    }

    fn unwrapped_address(addr: Address) -> Address {
        addr
    }
}
