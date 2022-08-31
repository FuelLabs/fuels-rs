contract;

struct MyStruct<T> {
    foo: T,
}

abi MyContract {
  fn identity(arg: MyStruct<u64>) -> MyStruct<u64>;
}

impl MyContract for Contract {
    fn identity(arg: MyStruct<u64>) -> MyStruct<u64> {
      arg
    }
}
