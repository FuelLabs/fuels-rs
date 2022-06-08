contract;

use std::address::Address;
use std::chain::auth::*;
use std::contract_id::ContractId;
use auth_testing_abi::*;
use std::result::*;
use std::assert::assert;
use std::identity::Identity;

impl AuthTesting for Contract {
    fn is_caller_external() -> bool {
        caller_is_external()
    }

    fn check_msg_sender(expected_id: Address) -> bool {
        let result: Result<Identity, AuthError> = msg_sender();
        let mut ret = false;
        if result.is_err() {
            ret = false;
        } else {
            let unwrapped = result.unwrap();
            if let Identity::Address(v) = unwrapped {
                assert(v == expected_id);
                ret = true;
            } else {
                ret = false;
            }
        };

        ret
    }
}
