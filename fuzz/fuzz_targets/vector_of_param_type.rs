#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate fuels_core;

use fuels_core::{codec::ABIEncoder, types::Token};
use fuels_test_helpers::{Config, WalletsConfig};

fuzz_target!(|param_types: Vec<ParamType>| {
    let max_depth = DecoderConfig::default().max_depth;
    let struct_of_param_types = ParamType::Struct{
        fields: param_types,
        generics: vec![],
    };
    let tuple = ParamType::Tuple(param_types);
    let enum_of_param_types = ParamType::Enum {
        variants: EnumVariants::new(param_types),
        generics: vec![],
    }

    let _ = struct_of_param_types.get_return_location();
    let _ = struct_of_param_types.compute_encoding_in_bytes();
    let _ = struct_of_param_types.children_need_extra_receipts();
    let _ = struct_of_param_types.validate_is_decodable(max_depth);
    let _ = struct_of_param_types.is_extra_receipt_needed(true);
    let _ = struct_of_param_types.is_extra_receipt_needed(false);
    let _ = struct_of_param_types.heap_inner_element_size(true);
    let _ = struct_of_param_types.heap_inner_element_size(false);

    let _ = tuple.get_return_location();
    let _ = tuple.compute_encoding_in_bytes();
    let _ = tuple.children_need_extra_receipts();
    let _ = tuple.validate_is_decodable(max_depth);
    let _ = tuple.is_extra_receipt_needed(true);
    let _ = tuple.is_extra_receipt_needed(false);
    let _ = tuple.heap_inner_element_size(true);
    let _ = tuple.heap_inner_element_size(false);

    let _ = enum_of_param_types.get_return_location();
    let _ = enum_of_param_types.compute_encoding_in_bytes();
    let _ = enum_of_param_types.children_need_extra_receipts();
    let _ = enum_of_param_types.validate_is_decodable(max_depth);
    let _ = enum_of_param_types.is_extra_receipt_needed(true);
    let _ = enum_of_param_types.is_extra_receipt_needed(false);
    let _ = enum_of_param_types.heap_inner_element_size(true);
    let _ = enum_of_param_types.heap_inner_element_size(false);
});
