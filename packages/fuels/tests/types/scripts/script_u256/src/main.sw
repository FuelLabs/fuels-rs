script;

use std::u256::U256;

fn main(arg: U256) -> U256 {
    arg + U256::from((6, 7, 8, 9))
}
