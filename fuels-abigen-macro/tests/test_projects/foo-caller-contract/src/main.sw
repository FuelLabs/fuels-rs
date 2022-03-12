contract;

use foo::FooContract;
use std::constants::ETH_ID;

abi FooCaller {
    fn call_foo_contract(value: bool) -> bool;
}

impl FooCaller for Contract {
    fn call_foo_contract(value: bool) -> bool {
        let foo_contract = abi(FooContract, 0xa3f8fc82d771bfee4622534d3d0437655af45ed7ed75af188b63aaa401801208);
        let response = foo_contract.foo{
            gas: 10000, 
            coins: 0, 
            asset_id: ETH_ID
        }
        (value);

        response
    }
}
