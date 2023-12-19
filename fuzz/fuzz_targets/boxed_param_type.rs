#![no_main]
use fuels::types::param_types::ParamType;
// mod utils;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|param_type: ParamType| {
    let array = ParamType::Array(Box::new(param_type.clone()), 10_000);
    let vector = ParamType::Vector(Box::new(param_type));

    todo!("util.rs file is currently missing");

    // utils::exercise_param_type(array);
    // utils::exercise_param_type(vector);
});
