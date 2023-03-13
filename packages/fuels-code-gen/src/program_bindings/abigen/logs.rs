use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use crate::program_bindings::{
    abi_types::{FullLoggedType, FullTypeDeclaration},
    resolved_type::resolve_type,
};

pub(crate) fn logs_printers_instantiation_code(
    contract_id: Option<TokenStream>,
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> TokenStream {
    let resolved_logs = resolve_logs(logged_types, shared_types);
    let log_id_param_type_pairs = generate_log_id_prettify_logs_pairs(&resolved_logs);
    let contract_id = contract_id
        .map(|id| quote! { ::core::option::Option::Some(#id) })
        .unwrap_or_else(|| quote! {::core::option::Option::None});
    quote! {::fuels::programs::logs::log_type_lookup(vec![#(#log_id_param_type_pairs),*], #contract_id)}
}

#[derive(Debug)]
struct ResolvedLog {
    log_id: u64,
    prettify_log_struct: TokenStream,
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

            let prettify_log_struct = quote! {
                ::fuels::programs::logs::PrettifyLog {
                    printer: ::fuels::core::decode_log::<#resolved_type>,
                    type_id: ::std::any::TypeId::of::<#resolved_type>()
                }
            };

            ResolvedLog {
                log_id: l.log_id,
                prettify_log_struct,
            }
        })
        .collect()
}

fn generate_log_id_prettify_logs_pairs(resolved_logs: &[ResolvedLog]) -> Vec<TokenStream> {
    resolved_logs
        .iter()
        .map(|r| {
            let id = r.log_id;
            let prettify_log_struct = &r.prettify_log_struct;

            quote! {
                (#id, #prettify_log_struct)
            }
        })
        .collect()
}
