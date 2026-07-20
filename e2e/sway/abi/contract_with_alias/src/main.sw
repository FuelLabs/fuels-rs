contract;

pub type MyAlias = Vec<b256>;
pub type MyU64 = u64;
pub type MyTuple = (MyU64, MyU64);
pub type MyArray = [MyTuple; 2];

abi MyContract {
    fn with_b256(b: b256) -> b256;
    fn with_myalias_vec() -> MyAlias;
    fn with_mytuple() -> MyTuple;
}

impl MyContract for Contract {
    fn with_b256(b: b256) -> b256 {
        b256::zero()
    }

    fn with_myalias_vec() -> MyAlias {
        MyAlias::new()
    }

    fn with_mytuple() -> MyTuple {
        (32, 64)
    }
}
