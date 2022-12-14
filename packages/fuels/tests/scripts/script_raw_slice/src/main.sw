script;

fn main(length: u64) -> raw_slice {
    let mut vec = Vec::new();
    vec.push(42);
    vec.push(1337);
    vec.push(1987);
    if (length > 3 ){
        let mut counter = 0;
        while counter < (length - 3) {
            vec.push(42234);
            counter = counter + 1;
        }
    }
    vec.as_raw_slice()
}
