contract;

use std::{
    address::Address,
    contract_id::ContractId,
    identity::Identity,
};

struct TestStruct {
    identity: Identity
}

enum TestEnum {
    EnumIdentity: Identity
}

const ADDR = 0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0;

abi MyContract {
    fn get_identity_address() -> Identity;
    fn get_identity_contract_id() -> Identity;
    fn get_struct_with_identity() -> TestStruct;
    fn get_enum_with_identity() -> TestEnum;
    fn get_identity_tuple() -> (TestStruct, TestEnum);
    fn input_identity(i: Identity) -> bool;
    fn input_struct_with_identity(s: TestStruct) -> bool;
    fn input_enum_with_identity(s: TestEnum) -> bool;
}

impl MyContract for Contract {
    fn get_identity_address() -> Identity {
        Identity::Address(~Address::from(ADDR))
    }

    fn get_identity_contract_id() -> Identity {
        Identity::ContractId(~ContractId::from(ADDR))
    }

    fn get_struct_with_identity() -> TestStruct {
        TestStruct{identity: Identity::Address(~Address::from(ADDR))}
    }

    fn get_enum_with_identity() -> TestEnum {
        TestEnum::EnumIdentity(Identity::ContractId(~ContractId::from(ADDR)))
    }

    fn get_identity_tuple() -> (TestStruct, TestEnum) {
        let s = TestStruct{identity: Identity::Address(~Address::from(ADDR))};
        let e = TestEnum::EnumIdentity(Identity::ContractId(~ContractId::from(ADDR)));
        (s,e)
    }


    fn input_identity(input: Identity) -> bool{
        if let Identity::Address(a) = input {
            return a == ~Address::from(ADDR);
        }
        false
    }

    fn input_struct_with_identity(input: TestStruct) -> bool {
        if let Identity::Address(a) = input.identity {
            return a == ~Address::from(ADDR);
        }
        false
    }

    fn input_enum_with_identity(input: TestEnum) -> bool {
        if let TestEnum::EnumIdentity(identity) = input {
            if let Identity::ContractId(c) = identity {
                return c == ~ContractId::from(ADDR);
            }
        }
        false
    }
}
