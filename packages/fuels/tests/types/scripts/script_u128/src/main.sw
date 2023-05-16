script;

use std::u128::U128;

fn main(arg: U128) -> U128 {
    log(arg);
    assert(arg == U128::from((1, 1)));

    U128::from((2, 2)) + arg
}
