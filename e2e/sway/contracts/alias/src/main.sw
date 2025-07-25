contract;

pub type MyAlias = b256;

abi MyContract {
    fn with_myalias(b: MyAlias) -> MyAlias;
}

impl MyContract for Contract {
    fn with_myalias(b: MyAlias) -> MyAlias {
        b256::zero()
    }
}
