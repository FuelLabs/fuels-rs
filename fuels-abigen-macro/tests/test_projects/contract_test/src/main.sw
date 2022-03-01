contract;

use std::*;
use core::*;
use std::storage::*;

abi TestContract {
  fn initialize_counter(gas: u64, coin: u64, color: b256, value: u64) -> u64;
  fn increment_counter(gas: u64, coin: u64, color: b256, value: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
  fn initialize_counter(gas: u64, coin: u64, color: b256, value: u64) -> u64 {
    store(COUNTER_KEY, value);
    value
  }
  fn increment_counter(gas: u64, coin: u64, color: b256, value: u64) -> u64 {
    let new_value = get::<u64>(COUNTER_KEY) + value;
    store(COUNTER_KEY, new_value);
    new_value
  }
}
