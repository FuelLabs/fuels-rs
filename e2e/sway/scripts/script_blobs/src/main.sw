script;

configurable {
    SECRET_NUMBER: u64 = 9000,
}

fn main() -> u64 {
    assert_eq(SECRET_NUMBER, 10001);
    return SECRET_NUMBER;
}
