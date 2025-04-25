predicate;

use std::{inputs::*};

struct SomeData {
    value: u64,
}

fn first_input_is_read_only_verified() -> bool {
    let first_input_type = input_type(0).unwrap();
    first_input_type == Input::ReadOnly(ReadOnlyInput::DataCoinPredicate)
}

fn read_only_input_value_matches_predicate_data(predicate_data: SomeData) -> bool {
    let input_data = input_data_coin_data::<SomeData>(0).unwrap();
    predicate_data.value == input_data.value
}

fn main(predicate_data: SomeData) -> bool {
    first_input_is_read_only_verified()
    && read_only_input_value_matches_predicate_data(predicate_data)
}
