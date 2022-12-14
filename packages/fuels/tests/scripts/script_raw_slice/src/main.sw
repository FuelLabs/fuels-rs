script;

fn main() -> raw_slice {
    let mut vec = Vec::new();
    vec.push(42);
    vec.push(1337);
    vec.push(1987);
    vec.push(987);
    vec.as_raw_slice()
}
