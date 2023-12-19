#![no_main]

// mod utils;
use fuels::core::codec::ABIEncoder;
use fuels::test_helpers::{Config, WalletsConfig};
use fuels::types::enum_variants::EnumVariants;
use fuels::types::param_types::ParamType;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|param_types: Vec<ParamType>| {
    let struct_of_param_types = ParamType::Struct {
        fields: param_types.clone(),
        generics: vec![],
    };
    let tuple = ParamType::Tuple(param_types.clone());
    let enum_of_param_types = ParamType::Enum {
        variants: EnumVariants::new(param_types).unwrap(),
        generics: vec![],
    };

    todo!("util.rs file is currently missing");
    // utils::exercise_param_type(struct_of_param_types);
    // utils::exercise_param_type(tuple);
    // utils::exercise_param_type(enum_of_param_types);
});
