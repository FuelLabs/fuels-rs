predicate;

fn main(a: u32, b: u64, c: Vec<u64>) -> bool {
    let number: u64 = c.get(2).unwrap();
    number == 42
}
