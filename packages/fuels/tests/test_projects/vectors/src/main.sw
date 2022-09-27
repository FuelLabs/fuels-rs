contract;

dep eq_impls;
dep data_structures;
dep utils;

use eq_impls::*;
use utils::*;
use std::vec::Vec;
use data_structures::{SomeEnum, SomeStruct};
use std::option::Option;
use std::assert::assert;

abi MyContract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32>;
    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) -> Vec<SomeStruct<u32>>;
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) -> SomeStruct<Vec<u32>>;
    fn array_in_vec(arg: Vec<[u64; 2]>) -> Vec<[u64; 2]>;
    fn vec_in_array(arg: [Vec<u32>; 2]) -> [Vec<u32>; 2];
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) -> Vec<SomeEnum<u32>>;
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) -> SomeEnum<Vec<u32>>;
    fn tuple_in_vec(arg: Vec<(u32, u32)>) -> Vec<(u32, u32)>;
    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) -> (Vec<u32>, Vec<u32>);

    fn vec_in_vec(arg: Vec<Vec<u32>>) -> Vec<Vec<u32>>;
}

impl MyContract for Contract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32> {
        let expected = expected_vec();

        assert(arg == expected);

        log_vec(expected);
        expected
    }

    fn vec_in_vec(arg: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
        let mut expected = ~Vec::new();
        expected.push(expected_vec());
        expected.push(expected_vec());

        assert(expected == arg);
        let mut i = 0;
        while i < expected.len() {
            match expected.get(i) {
                Option::Some(val) => {
                    log_vec(val);
                },
                _ => {
                    assert(false);
                }
            }
            i += 1;
        }
        log_vec(expected);

        expected
    }

    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) -> Vec<SomeStruct<u32>> {
        let mut expected = ~Vec::new();
        expected.push(SomeStruct { a: 0u32 });
        expected.push(SomeStruct { a: 1u32 });

        assert(arg == expected);
        log_vec(expected);
        expected
    }
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) -> SomeStruct<Vec<u32>> {
        let expected = SomeStruct {
            a: expected_vec(),
        };

        assert(arg.a == expected.a);
        log_vec(expected.a);
        expected
    }
    fn array_in_vec(arg: Vec<[u64; 2]>) -> Vec<[u64; 2]> {
        let mut expected = ~Vec::new();
        expected.push([0, 1]);
        expected.push([0, 1]);

        assert(arg == expected);
        log_vec(expected);
        expected
    }

    fn vec_in_array(arg: [Vec<u32>; 2]) -> [Vec<u32>; 2] {
        let expected = [
            expected_vec(),
            expected_vec(),
        ];

        assert(expected == arg);

        log_vec(expected[0]);
        log_vec(expected[1]);
        expected
    }
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) -> SomeEnum<Vec<u32>> {
        let vec = expected_vec();
        let expected = SomeEnum::a(vec);

        assert(expected == arg);
        log_vec(vec);
        expected
    }
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) -> Vec<SomeEnum<u32>> {
        let mut expected = ~Vec::new();
        expected.push(SomeEnum::a(0));
        expected.push(SomeEnum::a(1));

        assert(arg == expected);

        log_vec(expected);
        expected
    }

    fn tuple_in_vec(arg: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
        let mut expected = ~Vec::new();
        expected.push((0, 0));
        expected.push((1, 1));

        assert(arg == expected);
        log_vec(expected);
        expected
    }

    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) -> (Vec<u32>, Vec<u32>) {
        let expected = (
            expected_vec(),
            expected_vec(),
        );

        assert(arg == expected);

        log_vec(expected.0);
        log_vec(expected.1);
        expected
    }
}
