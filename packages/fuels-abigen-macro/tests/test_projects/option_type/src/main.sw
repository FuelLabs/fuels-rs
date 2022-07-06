contract;

use std::{address::Address, constants::BASE_ASSET_ID, identity::Identity, option::Option};

abi MyContract {
    fn get_some_address() -> Option<Identity>;
    fn get_none() -> Option<Identity>;
}

impl MyContract for Contract {
    fn get_some_address() -> Option<Identity> {
            Option::Some(Identity::Address(~Address::from(BASE_ASSET_ID)))
        }

    fn get_none() -> Option<Identity> {
        Option::None
    }
}
