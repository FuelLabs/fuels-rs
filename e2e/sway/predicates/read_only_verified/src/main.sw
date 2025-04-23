predicate;

struct Data {
    value: u64,
}

fn first_input_is_read_only_verified() -> bool {
    input_type(0) == Input::ReadOnly(ReadOnlyInput::DataCoinPredicate)
}

fn read_only_input_value_matches_predicate_data(predicate_data: u64) -> bool {
    let input_data = input_data_coin_data::<Data>(0).unwrap();
    predicate_data == input_data.value
}

fn main(predicate_data: u64) -> bool {
    first_input_is_read_only_verified()
    && read_only_input_value_matches_predicate_data(predicate_data)
}
