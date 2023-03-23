contract;
abi RawSliceContract {
    fn return_raw_slice(length: u64) -> raw_slice;
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
}
