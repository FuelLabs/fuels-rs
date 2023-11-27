contract;

#[allow(dead_code)]
enum SomeEnum<T> {
    A: bool,
    B: T,
}

abi MyContract {
    fn u256_sum_and_ret(some_u256: u256) -> u256;
    fn u256_in_enum_output() -> SomeEnum<u256>;
    fn u256_in_enum_input(some_enum: SomeEnum<u256>);
}

impl MyContract for Contract {
    fn u256_sum_and_ret(arg: u256) -> u256 {
        arg + 0x0000000000000003000000000000000400000000000000050000000000000006u256
    }

    fn u256_in_enum_output() -> SomeEnum<u256> {
        SomeEnum::B(
            0x0000000000000001000000000000000200000000000000030000000000000004u256,
        )
    }

    fn u256_in_enum_input(some_enum: SomeEnum<u256>) {
        if let SomeEnum::B(some_u256) = some_enum {
            require(
                some_u256 == 0x0000000000000002000000000000000300000000000000040000000000000005u256,
                "given u256 didn't match the expected u256",
            );
        } else {
            require(false, "enum was not of variant B: u256");
        }
    }
}
