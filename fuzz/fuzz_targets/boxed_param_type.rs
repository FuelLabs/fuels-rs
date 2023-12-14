#![no_main]
mod utils;
use libfuzzer_sys::fuzz_target;

use fuels_core::{codec::DecoderConfig, types::param_types::ParamType};

fuzz_target!(|param_type: ParamType| {
    let array = ParamType::Array(Box::new(param_type.clone()), 10_000);
    let vector = ParamType::Vector(Box::new(param_type));

    utils::exercise_param_type(array);
    utils::exercise_param_type(vector);
});
