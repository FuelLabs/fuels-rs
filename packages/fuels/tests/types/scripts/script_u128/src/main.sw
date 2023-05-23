script;

use std::u128::U128;

fn main(arg: U128) -> U128 {
    arg + U128::from((8, 2))
}
