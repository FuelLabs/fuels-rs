library auth_testing_abi;

use std::address::Address;
use std::contract_id::ContractId;
use std::chain::auth::*;
use std::result::*;

abi AuthTesting {
    fn is_caller_external() -> bool;
    fn check_msg_sender(expected_id: Address) -> bool;
}
