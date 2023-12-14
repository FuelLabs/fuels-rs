#![no_main]

mod utils;
use fuels_core::{codec::ABIEncoder, types::Token};
use fuels_test_helpers::{Config, WalletsConfig};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|param_types: Vec<ParamType>| {
    let struct_of_param_types = ParamType::Struct{
        fields: param_types,
        generics: vec![],
    };
    let tuple = ParamType::Tuple(param_types);
    let enum_of_param_types = ParamType::Enum {
        variants: EnumVariants::new(param_types),
        generics: vec![],
    }

    utils::exercise_param_type(struct_of_param_types);
    utils::exercise_param_type(tuple);
    utils::exercise_param_type(enum_of_param_types);
});
