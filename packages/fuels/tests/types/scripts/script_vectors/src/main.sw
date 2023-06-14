script;

mod data_structures;
mod eq_impls;
mod utils;

use eq_impls::*;
use utils::*;
use data_structures::*;

fn main(
    u32_vec: Vec<u32>,
    vec_in_vec: Vec<Vec<u32>>,
    struct_in_vec: Vec<SomeStruct<u32>>,
    vec_in_struct: SomeStruct<Vec<u32>>,
    array_in_vec: Vec<[u64; 2]>,
    vec_in_array: [Vec<u32>; 2],
    vec_in_enum: SomeEnum<Vec<u32>>,
    enum_in_vec: Vec<SomeEnum<u32>>,
    tuple_in_vec: Vec<(u32, u32)>,
    vec_in_tuple: (Vec<u32>, Vec<u32>),
    vec_in_a_vec_in_a_struct_in_a_vec: Vec<SomeStruct<Vec<Vec<u32>>>>,
) -> bool {
     {
        let exp_u32_vec = vec_from([0u32, 1u32, 2u32]);

        require(u32_vec == exp_u32_vec, "u32_vec_error");
    }
     {
        let mut exp_vec_in_vec = Vec::new();
        exp_vec_in_vec.push(vec_from([0u32, 1u32, 2u32]));
        exp_vec_in_vec.push(vec_from([0u32, 1u32, 2u32]));

        require(vec_in_vec == exp_vec_in_vec, "vec_in_vec err");
    }
     {
        let mut exp_struct_in_vec = Vec::new();
        exp_struct_in_vec.push(SomeStruct { a: 0u32 });
        exp_struct_in_vec.push(SomeStruct { a: 1u32 });

        require(struct_in_vec == exp_struct_in_vec, "struct_in_vec err");
    }
     {
        let exp_vec_in_struct = SomeStruct {
            a: vec_from([0u32, 1u32, 2u32]),
        };

        require(vec_in_struct.a == exp_vec_in_struct.a, "vec_in_struct err");
    }
     {
        let mut exp_array_in_vec = Vec::new();
        exp_array_in_vec.push([0, 1]);
        exp_array_in_vec.push([0, 1]);

        require(array_in_vec == exp_array_in_vec, "array_in_vec err");
    }
     {
        let exp_vec_in_array = [vec_from([0u32, 1u32, 2u32]), vec_from([0u32, 1u32, 2u32])];

        require(vec_in_array == exp_vec_in_array, "vec_in_array err");
    }
     {
        let exp_u32_vec = vec_from([0u32, 1u32, 2u32]);
        let exp_vec_in_enum = SomeEnum::a(exp_u32_vec);

        require(vec_in_enum == exp_vec_in_enum, "vec_in_enum err");
    }
     {
        let mut exp_enum_in_vec = Vec::new();
        exp_enum_in_vec.push(SomeEnum::a(0u32));
        exp_enum_in_vec.push(SomeEnum::a(1u32));

        require(enum_in_vec == exp_enum_in_vec, "enum_in_vec err");
    }
     {
        let mut exp_tuple_in_vec = Vec::new();
        exp_tuple_in_vec.push((0u32, 0u32));
        exp_tuple_in_vec.push((1u32, 1u32));

        require(tuple_in_vec == exp_tuple_in_vec, "tuple_in_vec err");
    }
     {
        let exp_vec_in_tuple = (vec_from([0u32, 1u32, 2u32]), vec_from([0u32, 1u32, 2u32]));

        require(vec_in_tuple == exp_vec_in_tuple, "vec_in_tuple err");
    }
     {
        let mut exp_vec_in_a_vec_in_a_struct_in_a_vec = Vec::new();

        let mut inner_vec_1 = Vec::new();

        let inner_inner_vec_1 = vec_from([0u32, 1u32, 2u32]);
        inner_vec_1.push(inner_inner_vec_1);

        let inner_inner_vec_2 = vec_from([3u32, 4u32, 5u32]);
        inner_vec_1.push(inner_inner_vec_2);

        exp_vec_in_a_vec_in_a_struct_in_a_vec.push(SomeStruct { a: inner_vec_1 });

        let mut inner_vec_2 = Vec::new();

        let inner_inner_vec_3 = vec_from([6u32, 7u32, 8u32]);
        inner_vec_2.push(inner_inner_vec_3);

        let inner_inner_vec_4 = vec_from([9u32, 10u32, 11u32]);
        inner_vec_2.push(inner_inner_vec_4);

        exp_vec_in_a_vec_in_a_struct_in_a_vec.push(SomeStruct { a: inner_vec_2 });

        require(vec_in_a_vec_in_a_struct_in_a_vec == exp_vec_in_a_vec_in_a_struct_in_a_vec, "vec_in_a_vec_in_a_struct_in_a_vec err");
    }

    true
}
