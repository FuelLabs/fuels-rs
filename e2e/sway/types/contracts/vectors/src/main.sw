contract;

pub mod data_structures;
mod eq_impls;
mod utils;

use eq_impls::*;
use utils::*;
use data_structures::*;

abi MyContract {
    fn u32_vec(arg: Vec<u32>);
    fn struct_in_vec(arg: Vec<SomeStruct<u32>>);
    fn vec_in_struct(arg: SomeStruct<Vec<u32>>);
    fn array_in_vec(arg: Vec<[u64; 2]>);
    fn vec_in_array(arg: [Vec<u32>; 2]);
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>);
    fn vec_in_enum(arg: SomeEnum<Vec<u32>>);
    fn tuple_in_vec(arg: Vec<(u32, u32)>);
    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>));
    fn vec_in_vec(arg: Vec<Vec<u32>>);
    fn vec_in_a_vec_in_a_struct_in_a_vec(arg: Vec<SomeStruct<Vec<Vec<u32>>>>);
}

impl MyContract for Contract {
    fn u32_vec(arg: Vec<u32>) {
        let expected = vec_from([0, 1, 2]);

        assert(arg == expected);
    }

    fn vec_in_vec(arg: Vec<Vec<u32>>) {
        let mut expected = Vec::new();
        expected.push(vec_from([0, 1, 2]));
        expected.push(vec_from([0, 1, 2]));

        assert_eq(arg, expected);
    }

    fn struct_in_vec(arg: Vec<SomeStruct<u32>>) {
        let mut expected = Vec::new();
        expected.push(SomeStruct { a: 0u32 });
        expected.push(SomeStruct { a: 1u32 });

        assert_eq(arg, expected);
    }

    fn vec_in_struct(arg: SomeStruct<Vec<u32>>) {
        let expected = SomeStruct {
            a: vec_from([0, 1, 2]),
        };

        assert_eq(arg, expected);
    }

    fn array_in_vec(arg: Vec<[u64; 2]>) {
        let mut expected = Vec::new();
        expected.push([0, 1]);
        expected.push([0, 1]);

        assert_eq(arg, expected);
    }

    fn vec_in_array(arg: [Vec<u32>; 2]) {
        let expected = [vec_from([0, 1, 2]), vec_from([0, 1, 2])];

        assert_eq(arg, expected);
    }

    fn vec_in_enum(arg: SomeEnum<Vec<u32>>) {
        let vec = vec_from([0, 1, 2]);
        let expected = SomeEnum::a(vec);

        assert_eq(arg, expected);
    }
    fn enum_in_vec(arg: Vec<SomeEnum<u32>>) {
        let mut expected = Vec::new();
        expected.push(SomeEnum::a(0u32));
        expected.push(SomeEnum::a(1u32));

        assert_eq(arg, expected);
    }

    fn tuple_in_vec(arg: Vec<(u32, u32)>) {
        let mut expected = Vec::new();
        expected.push((0u32, 0u32));
        expected.push((1u32, 1u32));

        assert_eq(arg, expected);
    }

    fn vec_in_tuple(arg: (Vec<u32>, Vec<u32>)) {
        let expected = (vec_from([0, 1, 2]), vec_from([0, 1, 2]));

        assert_eq(arg, expected);
    }

    fn vec_in_a_vec_in_a_struct_in_a_vec(arg: Vec<SomeStruct<Vec<Vec<u32>>>>) {
        let mut expected = Vec::new();

        let mut inner_vec_1 = Vec::new();

        let inner_inner_vec_1 = vec_from([0, 1, 2]);
        inner_vec_1.push(inner_inner_vec_1);

        let inner_inner_vec_2 = vec_from([3, 4, 5]);
        inner_vec_1.push(inner_inner_vec_2);

        expected.push(SomeStruct { a: inner_vec_1 });

        let mut inner_vec_2 = Vec::new();

        let inner_inner_vec_3 = vec_from([6, 7, 8]);
        inner_vec_2.push(inner_inner_vec_3);

        let inner_inner_vec_4 = vec_from([9, 10, 11]);
        inner_vec_2.push(inner_inner_vec_4);

        expected.push(SomeStruct { a: inner_vec_2 });

        assert_eq(arg, expected);
    }
}
