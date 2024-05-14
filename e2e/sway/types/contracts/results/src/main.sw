contract;

struct TestStruct {
    option: Option<Address>,
}

enum TestEnum {
    EnumOption: Option<Address>,
}

pub enum TestError {
    NoAddress: str[5],
    OtherError: (),
}

const ADDR = 0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0;

abi MyContract {
    fn get_ok_u64() -> Result<u64, TestError>;
    fn get_ok_address() -> Result<Address, TestError>;
    fn get_ok_struct() -> Result<TestStruct, TestError>;
    fn get_ok_enum() -> Result<TestEnum, TestError>;
    fn get_ok_tuple() -> Result<(TestStruct, TestEnum), TestError>;
    fn get_error() -> Result<Address, TestError>;
    fn input_ok(ok_address: Result<Address, TestError>) -> bool;
    fn input_error(test_error: Result<Address, TestError>) -> bool;
}

impl MyContract for Contract {
    fn get_ok_u64() -> Result<u64, TestError> {
        Result::Ok(10)
    }

    fn get_ok_address() -> Result<Address, TestError> {
        Result::Ok(Address::from(ADDR))
    }

    fn get_ok_struct() -> Result<TestStruct, TestError> {
        Result::Ok(TestStruct {
            option: Option::Some(Address::from(ADDR)),
        })
    }

    fn get_ok_enum() -> Result<TestEnum, TestError> {
        Result::Ok(TestEnum::EnumOption(Option::Some(Address::from(ADDR))))
    }

    fn get_ok_tuple() -> Result<(TestStruct, TestEnum), TestError> {
        let s = TestStruct {
            option: Option::Some(Address::from(ADDR)),
        };
        let e = TestEnum::EnumOption(Option::Some(Address::from(ADDR)));
        Result::Ok((s, e))
    }

    fn get_error() -> Result<Address, TestError> {
        Result::Err(TestError::NoAddress(__to_str_array("error")))
    }

    fn input_ok(ok_address: Result<Address, TestError>) -> bool {
        if let Result::Ok(a) = ok_address {
            return a == Address::from(ADDR);
        }
        false
    }

    fn input_error(test_result: Result<Address, TestError>) -> bool {
        if let Result::Err(test_error) = test_result {
            if let TestError::NoAddress(_err_msg) = test_error {
                return true;
            }
        }
        false
    }
}
