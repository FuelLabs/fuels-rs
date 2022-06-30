contract;

use std::storage::get;

abi TestContract {
  #[storage(read)]
  fn get_value(key: b256) -> b256;
}

impl TestContract for Contract {
  #[storage(read)]
  fn get_value(key: b256) -> b256 {
    get::<b256>(key)
  }
}
