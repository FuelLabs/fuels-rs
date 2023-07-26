use fuel_abi_types::abi::full_program::FullLoggedType;
use proc_macro2::TokenStream;
use quote::quote;

use crate::program_bindings::resolved_type::TypeResolver;

pub(crate) fn log_formatters_instantiation_code(
    contract_id: TokenStream,
    logged_types: &[FullLoggedType],
) -> TokenStream {
    let resolved_logs = resolve_logs(logged_types);
    let log_id_log_formatter_pairs = generate_log_id_log_formatter_pairs(&resolved_logs);
    quote! {::fuels::programs::logs::log_formatters_lookup(vec![#(#log_id_log_formatter_pairs),*], #contract_id)}
}

#[derive(Debug)]
struct ResolvedLog {
    log_id: u64,
    log_formatter: TokenStream,
}

/// Reads the parsed logged types from the ABI and creates ResolvedLogs
fn resolve_logs(logged_types: &[FullLoggedType]) -> Vec<ResolvedLog> {
    logged_types
        .iter()
        .map(|l| {
            let resolved_type = TypeResolver::default()
                .resolve(&l.application)
                .expect("Failed to resolve log type");

            ResolvedLog {
                log_id: l.log_id,
                log_formatter: quote! {
                    ::fuels::programs::logs::LogFormatter::new::<#resolved_type>()
                },
            }
        })
        .collect()
}

fn generate_log_id_log_formatter_pairs(resolved_logs: &[ResolvedLog]) -> Vec<TokenStream> {
    resolved_logs
        .iter()
        .map(|r| {
            let id = r.log_id;
            let log_formatter = &r.log_formatter;

            quote! {
                (#id, #log_formatter)
            }
        })
        .collect()
}
