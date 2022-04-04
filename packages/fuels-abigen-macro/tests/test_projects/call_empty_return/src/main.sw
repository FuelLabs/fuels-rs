contract;

use std::storage::store;
use std::storage::get;
use std::chain::log_u64;
use std::chain::log_u8;

abi TestContract {
  fn store_value(val: u64);
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
  fn store_value(val: u64) {
    store(COUNTER_KEY, val);
  }
}
