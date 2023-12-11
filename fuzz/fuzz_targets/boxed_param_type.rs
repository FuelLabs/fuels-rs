#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate fuels_core;

use fuels_core::{codec::DecoderConfig, types::param_types::ParamType};

fuzz_target!(|param_type: ParamType| {
    let max_depth = DecoderConfig::default().max_depth;
    let array = ParamType::Array(Box::new(param_type), 10_000);
    let vector = ParamType::Vector(Box::new(param_type));

    let _ = array.get_return_location();
    let _ = array.compute_encoding_in_bytes();
    let _ = array.children_need_extra_receipts();
    let _ = array.validate_is_decodable(max_depth);
    let _ = array.is_extra_receipt_needed(true);
    let _ = array.is_extra_receipt_needed(false);
    let _ = array.heap_inner_element_size(true);
    let _ = array.heap_inner_element_size(false);

    let _ = vector.get_return_location();
    let _ = vector.compute_encoding_in_bytes();
    let _ = vector.children_need_extra_receipts();
    let _ = vector.validate_is_decodable(max_depth);
    let _ = vector.is_extra_receipt_needed(true);
    let _ = vector.is_extra_receipt_needed(false);
    let _ = vector.heap_inner_element_size(true);
    let _ = vector.heap_inner_element_size(false);
});
