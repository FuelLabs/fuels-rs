contract;

use std::*;
use core::*;
use std::storage::*;

abi TestContract {
  fn get_id() -> b256;
  fn get_small_string() -> str[8];
  fn get_large_string() -> str[9];
}

impl TestContract for Contract {
  fn get_id() -> b256 {
    0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
  }

  fn get_small_string() -> str[8] {
    let my_string: str[8] = "gggggggg";
    my_string
  }
  
  fn get_large_string() -> str[9] {
    let my_string: str[9] = "ggggggggg";
    my_string
  }
}
