contract;

dep eq_impls;
dep data_structures;
dep utils;

use eq_impls::*;
use utils::*;
use data_structures::*;
use std::logging::log;

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

    fn vec_in_a_vec_in_a_struct_in_a_vec(arg: Vec<SomeStruct<Vec<Vec<u32>>>>) -> Vec<SomeStruct<Vec<Vec<u32>>>>;

    // examples
    fn returning_a_vec() -> Vec<u32>;
    fn returning_type_w_nested_vectors() -> Parent;
    fn returning_immediately_nested_vectors() -> Vec<Vec<u32>>;
}

impl MyContract for Contract {
    fn u32_vec(arg: Vec<u32>) -> Vec<u32> {
        let expected = vec_from([0, 1, 2]);

        assert(arg == expected);

        log_vec(expected);
        expected
    }

    fn vec_in_vec(arg: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
        let mut expected = ~Vec::new();
        expected.push(vec_from([0, 1, 2]));
        expected.push(vec_from([0, 1, 2]));

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
            a: vec_from([0, 1, 2]),
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
            vec_from([0, 1, 2]),
            vec_from([0, 1, 2]),
        ];

        assert(expected == arg);

        log_vec(expected[0]);
        log_vec(expected[1]);
        expected
    }
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) -> SomeEnum<Vec<u32>> {
        let vec = vec_from([0, 1, 2]);
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
            vec_from([0, 1, 2]),
            vec_from([0, 1, 2]),
        );

        assert(arg == expected);

        log_vec(expected.0);
        log_vec(expected.1);
        expected
    }
    fn vec_in_a_vec_in_a_struct_in_a_vec(
        arg: Vec<SomeStruct<Vec<Vec<u32>>>>,
    ) -> Vec<SomeStruct<Vec<Vec<u32>>>> {
        let mut expected = ~Vec::new();

        let mut inner_vec_1 = ~Vec::new();

        let inner_inner_vec_1 = vec_from([0, 1, 2]);
        inner_vec_1.push(inner_inner_vec_1);

        let inner_inner_vec_2 = vec_from([3, 4, 5]);
        inner_vec_1.push(inner_inner_vec_2);

        expected.push(SomeStruct { a: inner_vec_1 });

        let mut inner_vec_2 = ~Vec::new();

        let inner_inner_vec_3 = vec_from([6, 7, 8]);
        inner_vec_2.push(inner_inner_vec_3);

        let inner_inner_vec_4 = vec_from([9, 10, 11]);
        inner_vec_2.push(inner_inner_vec_4);

        expected.push(SomeStruct { a: inner_vec_2 });

        assert(arg == expected);

        log_vec(inner_inner_vec_1);
        log_vec(inner_inner_vec_2);
        log_vec(inner_vec_1);

        log_vec(inner_inner_vec_3);
        log_vec(inner_inner_vec_4);
        log_vec(inner_vec_2);

        log_vec(expected);

        expected
    }

    //ANCHOR: sway_returning_a_vec
    fn returning_a_vec() -> Vec<u32> {
        let mut vec = ~Vec::new();
        vec.push(1);
        vec.push(2);

        let mut i = 0;
        while i < vec.len() {
            log(vec.get(i).unwrap());
            i += 1;
        }

        vec
    }
    //ANCHOR_END: sway_returning_a_vec
    //ANCHOR: sway_returning_type_w_nested_vectors
    fn returning_type_w_nested_vectors() -> Parent {
        let mut grandchild_vec = ~Vec::new();
        grandchild_vec.push(0);

        let mut child_info_vec = ~Vec::new();
        child_info_vec.push(1);

        let child = Child {
            grandchild: grandchild_vec,
            info: child_info_vec,
        };

        let mut parent_info_vec = ~Vec::new();
        parent_info_vec.push(2);

        let parent = Parent {
            child,
            info: parent_info_vec,
        };

        log_vec(grandchild_vec);
        log_vec(child_info_vec);
        log_vec(parent_info_vec);

        parent
    }
    //ANCHOR_END: sway_returning_type_w_nested_vectors
    //ANCHOR: sway_returning_immediately_nested_vectors
    fn returning_immediately_nested_vectors() -> Vec<Vec<u32>> {
        let mut parent_vec = ~Vec::new();

        let mut inner_vec_1 = ~Vec::new();
        inner_vec_1.push(1);
        parent_vec.push(inner_vec_1);

        let mut inner_vec_2 = ~Vec::new();
        inner_vec_2.push(2);
        parent_vec.push(inner_vec_2);

        log_vec(inner_vec_1);
        log_vec(inner_vec_2);
        log_vec(parent_vec);

        parent_vec
    }
    //ANCHOR_END: sway_returning_immediately_nested_vectors
}
