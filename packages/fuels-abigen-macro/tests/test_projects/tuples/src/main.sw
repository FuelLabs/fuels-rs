contract;

struct Person {
    name: str[4],
}

enum State {
    A: (),
    B: (),
    C: (),
}

abi MyContract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64);
    fn returns_struct_in_tuple(input: (u64, Person)) -> (u64, Person);
    fn returns_enum_in_tuple(input: (u64, State)) -> (u64, State);
}

impl MyContract for Contract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64) {
        input
    }

    fn returns_struct_in_tuple(input: (u64, Person)) -> (u64, Person) {
        input
    }

    fn returns_enum_in_tuple(input: (u64, State)) -> (u64, State) {
        input
    }
}
