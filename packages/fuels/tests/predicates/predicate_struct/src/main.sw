predicate;

use std::inputs::input_predicate_data;

struct Validation {
    has_account: bool,
    total_complete: u64,
}

fn main(input: Validation) -> bool {
    let expected_has_account: bool = true;
    let expected_total_complete: u64 = 100;

    input.has_account == expected_has_account && input.total_complete == expected_total_complete
}
