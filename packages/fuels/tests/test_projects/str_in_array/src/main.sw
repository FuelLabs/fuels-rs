contract;

abi TestContract {
    fn take_array_string_shuffle(a: [str[3]; 3]) -> [str[3]; 3];
    fn take_array_string_return_single(a: [str[3]; 3]) -> [str[3]; 1];
    fn take_array_string_return_single_element(a: [str[3]; 3]) -> str[3];
}

impl TestContract for Contract {
    fn take_array_string_shuffle(a: [str[3]; 3]) -> [str[3]; 3] {
        [a[2], a[0], a[1]]
    }

    fn take_array_string_return_single(a: [str[3]; 3]) -> [str[3]; 1] {
        [a[0]]
    }

    fn take_array_string_return_single_element(a: [str[3]; 3]) -> str[3] {
        a[1]
    }
}
