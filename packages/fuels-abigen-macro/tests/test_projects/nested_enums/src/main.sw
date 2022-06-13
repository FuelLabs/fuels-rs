contract;

use std::{
    address::Address,
    constants::NATIVE_ASSET_ID,
    identity::Identity,
    option::Option,
};

abi MyContract {
    fn some_address() -> Option<Identity>;
    fn none() -> Option<Identity>;
}

impl MyContract for Contract {
    fn some_address() -> Option<Identity> {
        Option::Some(Identity::Address(~Address::from(NATIVE_ASSET_ID)))
    }
    fn none() -> Option<Identity> {
        Option::None
    }
}
