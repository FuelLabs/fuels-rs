contract;

use std::{address::Address, constants::ZERO_B256, identity::Identity, option::Option};

abi MyContract {
    fn get_some_address() -> Option<Identity>;
    fn get_none() -> Option<Identity>;
}

impl MyContract for Contract {
    fn get_some_address() -> Option<Identity> {
            Option::Some(Identity::Address(~Address::from(ZERO_B256)))
        }

    fn get_none() -> Option<Identity> {
        Option::None
    }
}
