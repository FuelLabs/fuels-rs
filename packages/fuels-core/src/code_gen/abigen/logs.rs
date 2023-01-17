use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use fuel_abi_types::program_abi::ResolvedLog;

use crate::code_gen::{
    abi_types::{FullLoggedType, FullTypeDeclaration},
    resolved_type::resolve_type,
    utils::single_param_type_call,
};

pub(crate) fn logs_hashmap_instantiation_code(
    contract_id: Option<TokenStream>,
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> TokenStream {
    let resolved_logs = resolve_logs(logged_types, shared_types);
    let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);
    let contract_id = contract_id
        .map(|id| quote! { ::std::option::Option::Some(#id) })
        .unwrap_or_else(|| quote! {::std::option::Option::None});
    quote! {::fuels::core::get_logs_hashmap(&[#(#log_id_param_type_pairs),*], #contract_id)}
}

/// Reads the parsed logged types from the ABI and creates ResolvedLogs
fn resolve_logs(
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Vec<ResolvedLog> {
    logged_types
        .iter()
        .map(|l| {
            let resolved_type =
                resolve_type(&l.application, shared_types).expect("Failed to resolve log type");
            let param_type_call = single_param_type_call(&resolved_type);
            let resolved_type_name = TokenStream::from(resolved_type);

            ResolvedLog {
                log_id: l.log_id,
                param_type_call,
                resolved_type_name,
            }
        })
        .collect()
}

fn generate_log_id_param_type_pairs(resolved_logs: &[ResolvedLog]) -> Vec<TokenStream> {
    resolved_logs
        .iter()
        .map(|r| {
            let id = r.log_id;
            let param_type_call = &r.param_type_call;

            quote! {
                (#id, #param_type_call)
            }
        })
        .collect()
}
