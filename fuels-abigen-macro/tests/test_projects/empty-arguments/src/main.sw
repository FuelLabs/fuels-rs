contract;

use std::storage::store;
use std::storage::get;

abi TestContract {
  fn method_with_empty_parenthesis_argument(gas_: u64, amount_: u64, color: b256, an_argument_name: ()) -> u64;
  fn method_with_empty_string_argument(gas_: u64, amount_: u64, color: b256, another_argument_name: ()) -> u64;
  fn method_with_empty_argument(gas_: u64, amount_: u64, color: b256, input: ()) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
  fn method_with_empty_parenthesis_argument(gas_: u64, amount_: u64, color: b256, an_argument_name: ()) -> u64 {
    21
  }
  fn method_with_empty_string_argument(gas_: u64, amount_: u64, color: b256, another_argument_name: ()) -> u64 {
    42
  }
  fn method_with_empty_argument(gas_: u64, amount_: u64, color: b256, bimbam: ()) -> u64 {
    63
  }
}
