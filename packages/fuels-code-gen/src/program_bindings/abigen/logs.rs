use proc_macro2::TokenStream;
use quote::quote;

use crate::program_bindings::{
    abi_types::FullLoggedType, resolved_type::TypeResolver, utils::single_param_type_call,
};

pub(crate) fn logs_lookup_instantiation_code(
    contract_id: Option<TokenStream>,
    logged_types: &[FullLoggedType],
) -> TokenStream {
    let resolved_logs = resolve_logs(logged_types);
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
fn resolve_logs(logged_types: &[FullLoggedType]) -> Vec<ResolvedLog> {
    logged_types
        .iter()
        .map(|l| {
            let type_application = &l.application;
            let resolved_type = TypeResolver::default()
                .resolve(type_application)
                .expect("Failed to resolve log type");
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
