use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use crate::program_bindings::{
    abi_types::{FullLoggedType, FullTypeDeclaration},
    resolved_type::resolve_type,
    utils::single_param_type_call,
};

pub(crate) fn logs_lookup_instantiation_code(
    contract_id: Option<TokenStream>,
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> TokenStream {
    let resolved_logs = resolve_logs(logged_types, shared_types);
    let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);
    let contract_id = contract_id
        .map(|id| quote! { ::core::option::Option::Some(#id) })
        .unwrap_or_else(|| quote! {::core::option::Option::None});
    quote! {::fuels::programs::logs::log_type_lookup(&[#(#log_id_param_type_pairs),*], #contract_id)}
}

#[derive(Debug)]
struct ResolvedLog {
    log_id: u64,
    param_type_call: TokenStream,
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

            ResolvedLog {
                log_id: l.log_id,
                param_type_call,
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
