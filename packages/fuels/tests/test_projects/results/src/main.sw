contract;

use std::{
    address::Address,
    constants::ZERO_B256,
    result::Result,
    option::Option
};

struct TestStruct {
    option: Option<Address>
}

enum TestEnum {
    EnumOption: Option<Address>
}

pub enum TestError {
    NoAddress: str[5],
}

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
        Result::Ok(~Address::from(ZERO_B256))
    }

    fn get_ok_struct() -> Result<TestStruct, TestError> {
        Result::Ok(TestStruct{option: Option::Some(~Address::from(ZERO_B256))})
    }

    fn get_ok_enum() -> Result<TestEnum, TestError> {
        Result::Ok(TestEnum::EnumOption(Option::Some(~Address::from(ZERO_B256))))
    }

    fn get_ok_tuple() -> Result<(TestStruct, TestEnum), TestError> {
        let s = TestStruct{option: Option::Some(~Address::from(ZERO_B256))};
        let e = TestEnum::EnumOption(Option::Some(~Address::from(ZERO_B256)));
        Result::Ok((s,e))
    }

    fn get_error() -> Result<Address, TestError> {
        Result::Err(TestError::NoAddress("error"))
    }

    fn input_ok(ok_address: Result<Address, TestError>) -> bool {
        if let Result::Ok(a) = ok_address {
            return a == ~Address::from(ZERO_B256);
        }
        false
    }

    fn input_error(test_result: Result<Address, TestError>) -> bool {
        if let Result::Err(test_error) = test_result {
            if let TestError::NoAddress(_err_msg) = test_error {
                return true
            }
        }
        false
    }
}
