contract;

use std::*;
use core::*;
use std::storage::*;

struct MyStruct {
  foo: u8,
  bar: bool,
}

abi TestContract {
  fn is_even(gas: u64, coin: u64, color: b256, value: u64) -> bool;
  fn return_my_string(gas: u64, coin: u64, color: b256, value: str[4]) -> str[4];
  fn return_my_struct(gas: u64, coin: u64, color: b256, value: MyStruct) -> MyStruct;
  
}

impl TestContract for Contract {
  fn is_even(gas: u64, coin: u64, color: b256, value: u64) -> bool {
    if (value / 2) * 2 == value {
      true
    } else {
      false
    }
  }
  fn return_my_string(gas: u64, coin: u64, color: b256, value: str[4]) -> str[4] {
    value
  }


  fn return_my_struct(gas: u64, coin: u64, color: b256, value: MyStruct) -> MyStruct {
    value
  }
  
}
