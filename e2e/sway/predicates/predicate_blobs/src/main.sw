predicate;

configurable {
    SECRET_NUMBER: u64 = 9000,
}

fn main(arg1: u8, arg2: u8) -> bool {
    arg1 == 1 && arg2 == 19 && SECRET_NUMBER == 10001
}
