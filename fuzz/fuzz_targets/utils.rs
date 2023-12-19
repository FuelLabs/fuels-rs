use fuels::{core::codec::DecoderConfig, types::param_types::ParamType};
pub fn exercise_param_type(param_type: ParamType) {
    let max_depth = DecoderConfig::default().max_depth;
    let _ = param_type.get_return_location();
    let _ = param_type.compute_encoding_in_bytes();
    let _ = param_type.children_need_extra_receipts();
    let _ = param_type.validate_is_decodable(max_depth);
    let _ = param_type.is_extra_receipt_needed(true);
    let _ = param_type.is_extra_receipt_needed(false);
    let _ = param_type.heap_inner_element_size(true);
    let _ = param_type.heap_inner_element_size(false);
}
