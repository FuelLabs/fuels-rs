script;

dep eq_impls;
dep data_structures;
dep utils;

use eq_impls::*;
use utils::*;
use data_structures::*;

fn main(
    u32_vec: Vec<u32>,
    u32_vec2: Vec<u32>,
    u32_vec3: Vec<u32>,
    u32_vec4: Vec<u32>,
    vec_in_vec: Vec<Vec<u32>>,
    struct_in_vec: Vec<SomeStruct<u32>>,
    vec_in_struct: SomeStruct<Vec<u32>>,
    array_in_vec: Vec<[u64; 2]>,
    vec_in_array: [Vec<u32>; 2],
) -> bool {
    let exp_u32_vec = vec_from([0, 1, 2]);

    let mut exp_vec_in_vec = Vec::new();
    exp_vec_in_vec.push(vec_from([0, 1, 2]));
    exp_vec_in_vec.push(vec_from([0, 1, 2]));

    let mut exp_struct_in_vec = Vec::new();
    exp_struct_in_vec.push(SomeStruct { a: 0u32 });
    exp_struct_in_vec.push(SomeStruct { a: 1u32 });

    let exp_vec_in_struct = SomeStruct {
        a: vec_from([0, 1, 2]),
    };

    let mut exp_array_in_vec = Vec::new();
    exp_array_in_vec.push([0, 1]);
    exp_array_in_vec.push([0, 1]);

    let exp_vec_in_array = [vec_from([0, 1, 2]), vec_from([0, 1, 2])];

    let r: u8 = u32_vec.get(1).unwrap();
    require(u32_vec == exp_u32_vec, r);
    require(vec_in_vec == exp_vec_in_vec, "vec_in_vec err");
    require(struct_in_vec == exp_struct_in_vec, "struct_in_vec err");
    require(vec_in_struct.a == exp_vec_in_struct.a, "vec_in_struct err");
    require(array_in_vec == exp_array_in_vec, "array_in_vec err");
    require(vec_in_array == exp_vec_in_array, "vec_in_array err");

    true
}
