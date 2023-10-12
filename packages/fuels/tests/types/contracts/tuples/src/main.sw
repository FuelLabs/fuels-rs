contract;

use std::{constants::ZERO_B256, hash::*};

struct Person {
    name: str[4],
}

#[allow(dead_code)]
enum State {
    A: (),
    B: (),
    C: (),
}

impl Hash for str[4] {
    fn hash(self, ref mut state: Hasher) {
        state.write_str(from_str_array(self));
    }
}

abi MyContract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64);
    fn returns_struct_in_tuple(input: (u64, Person)) -> (u64, Person);
    fn returns_enum_in_tuple(input: (u64, State)) -> (u64, State);
    fn single_element_tuple(input: (u64, )) -> (u64, );
    fn tuple_with_b256(p: (b256, u8)) -> (b256, u8);
}

impl MyContract for Contract {
    fn returns_tuple(input: (u64, u64)) -> (u64, u64) {
        let expected = (1u64, 2u64);

        assert(expected.0 == input.0);
        assert(expected.1 == input.1);

        expected
    }

    fn returns_struct_in_tuple(input: (u64, Person)) -> (u64, Person) {
        let expected = (42, Person {
            name: __to_str_array("Jane"),
        });
        assert(input.0 == expected.0);
        assert(sha256(from_str_array(input.1.name)) == sha256(from_str_array(expected.1.name)));

        expected
    }

    fn returns_enum_in_tuple(input: (u64, State)) -> (u64, State) {
        let expected = (42, State::A);

        assert(input.0 == expected.0);

        match input.1 {
            State::A => {},
            _ => {
                assert(false)
            }
        };

        expected
    }

    fn single_element_tuple(input: (u64, )) -> (u64, ) {
        let expected = (123u64, );

        assert(expected.0 == input.0);

        expected
    }

    fn tuple_with_b256(p: (b256, u8)) -> (b256, u8) {
        let expected = (ZERO_B256, 10u8);

        assert(p.0 == expected.0);
        assert(p.1 == expected.1);

        expected
    }
}
