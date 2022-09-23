contract;

use std::*;
use core::*;

struct MyStruct {
  foo: u8,
  bar: bool,
}

abi TestContract {
  fn is_even(value: u64) -> bool;
  fn return_my_string(value: str[4]) -> str[4];
  fn return_my_struct(value: MyStruct) -> MyStruct;
  
}

impl TestContract for Contract {
  fn is_even(value: u64) -> bool {
    if (value / 2) * 2 == value {
      true
    } else {
      false
    }
  }
  fn return_my_string(value: str[4]) -> str[4] {
    value
  }


  fn return_my_struct(value: MyStruct) -> MyStruct {
    value
  }
  
}
