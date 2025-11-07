contract;

pub type MyAlias = b256;
pub type MyU64 = u64;
pub type MyTuple = (MyU64, MyU64);

abi MyContract {
    fn with_myalias(b: MyAlias) -> MyAlias;
    fn with_mytuple() -> MyTuple;
}

impl MyContract for Contract {
    fn with_myalias(b: MyAlias) -> MyAlias {
        b256::zero()
    }

    fn with_mytuple() -> MyTuple {
        (32, 64)
    }
}
