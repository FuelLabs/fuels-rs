contract;

pub struct StructOne {
    foo: u64,
}

pub struct StructTwo {
    bar: u64,
}

abi MyTest {
    fn something(gas_: u64, amount_: u64, color_: b256, input: StructTwo) -> u64;
    fn something_else(gas_: u64, amount_: u64, color_: b256, input: StructOne) -> u64;
}

impl MyTest for Contract {
    fn something(gas_: u64, amount_: u64, color_: b256, input: StructOne) -> u64 {
        let v = input.foo; 
        v + 1
    }    

    fn something_else(gas_: u64, amount_: u64, color_: b256, input: StructTwo) -> u64 {
        let v = input.bar; 
        v - 1
    }    
}