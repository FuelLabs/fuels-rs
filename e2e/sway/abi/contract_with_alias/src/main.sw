contract;

pub type MyAlias = Vec<b256>;
//pub type MyAliasGeneric<T> = Vec<T>;

abi MyContract {
    fn with_b256(b: b256) -> b256;
    fn with_myalias_vec() -> MyAlias;
}

impl MyContract for Contract {
    fn with_b256(b: b256) -> b256 {
        b256::zero()
    }

    fn with_myalias_vec() -> MyAlias {
        MyAlias::new()
    }
}
