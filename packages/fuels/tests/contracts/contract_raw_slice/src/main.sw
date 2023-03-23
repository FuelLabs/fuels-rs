contract;
abi RawSliceContract {
    fn return_raw_slice(length: u64) -> raw_slice;
    fn accept_raw_slice(slice: raw_slice);
}

fn validate_raw_slice(input: raw_slice) {
    let vec: Vec<u64> = Vec::from(input);
    require(vec.len() == 3, "raw slice len is not 3");
    require(vec.get(2).unwrap() == 42, "expected 3. slice entry to be 42");
    require(vec.get(1).unwrap() == 41, "expected 2. slice entry to be 41");
    require(vec.get(0).unwrap() == 40, "expected 1. slice entry to be 40");
}

impl RawSliceContract for Contract {
    fn return_raw_slice(length: u64) -> raw_slice {
        let mut vec = Vec::new();
        let mut counter = 0;
        while counter < length {
            vec.push(counter);
            counter = counter + 1;
        }
        vec.as_raw_slice()
    }
    fn accept_raw_slice(slice: raw_slice){
        validate_raw_slice(slice);
    }
}
