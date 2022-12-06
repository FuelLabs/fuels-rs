predicate;

use std::inputs::input_predicate_data;

fn main() -> bool {
    let guessed_number: u64 = input_predicate_data(0);
    if guessed_number == 42 {
        return true;
    }
    false
}
