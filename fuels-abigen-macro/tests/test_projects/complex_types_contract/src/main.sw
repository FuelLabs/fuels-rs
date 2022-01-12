contract;

use std::storage::store;
use std::storage::get;
use std::chain::log_u64;
use std::chain::log_u8;

struct CounterConfig {
  dummy: bool,
  initial_value: u64,
}

abi TestContract {
  fn initialize_counter(gas_: u64, amount_: u64, color_: b256, config: CounterConfig) -> u64;
  fn increment_counter(gas_: u64, amount_: u64, color_: b256, amount: u64) -> u64;
}

const COUNTER_KEY = 0x0000000000000000000000000000000000000000000000000000000000000000;

impl TestContract for Contract {
  fn initialize_counter(gas_: u64, amount_: u64, color_: b256, config: CounterConfig) -> u64 {
    let value = config.initial_value;
    store(COUNTER_KEY, value);
    value
  }
  fn increment_counter(gas_: u64, amount_: u64, color_: b256, amount: u64) -> u64 {
    let value = get::<u64>(COUNTER_KEY) + amount;
    store(COUNTER_KEY, value);
    value
  }
}
