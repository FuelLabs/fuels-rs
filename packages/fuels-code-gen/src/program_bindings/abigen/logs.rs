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
    quote! {::fuels::core::codec::log_formatters_lookup(vec![#(#log_id_log_formatter_pairs),*], #contract_id)}
}

#[derive(Debug)]
struct ResolvedLog {
    log_id: String,
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

            let is_error_type = l
                .application
                .type_decl
                .components
                .iter()
                .any(|component| component.error_message.is_some());

            let log_formatter = if is_error_type {
                quote! {
                    ::fuels::core::codec::LogFormatter::new_error::<#resolved_type>()
                }
            } else {
                quote! {
                    ::fuels::core::codec::LogFormatter::new_log::<#resolved_type>()
                }
            };

            ResolvedLog {
                log_id: l.log_id.clone(),
                log_formatter,
            }
        })
        .collect()
}

fn generate_log_id_log_formatter_pairs(
    resolved_logs: &[ResolvedLog],
) -> impl Iterator<Item = TokenStream> {
    resolved_logs.iter().map(|r| {
        let id = &r.log_id;
        let log_formatter = &r.log_formatter;

        quote! {
            (#id.to_string(), #log_formatter)
        }
    })
}

pub(crate) fn generate_id_error_codes_pairs(
    error_codes: impl IntoIterator<Item = (u64, fuel_abi_types::abi::program::ErrorDetails)>,
) -> impl Iterator<Item = TokenStream> {
    error_codes.into_iter().map(|(id, ed)| {
        let pkg = ed.pos.pkg;
        let file = ed.pos.file;
        let line = ed.pos.line;
        let column = ed.pos.column;

        let log_id = ed.log_id.map_or(
            quote! {::core::option::Option::None},
            |l| quote! {::core::option::Option::Some(#l.to_string())},
        );
        let msg = ed.msg.map_or(
            quote! {::core::option::Option::None},
            |m| quote! {::core::option::Option::Some(#m.to_string())},
        );

        quote! {
            (#id,
             ::fuels::core::codec::ErrorDetails::new(
                    #pkg.to_string(), #file.to_string(), #line, #column, #log_id, #msg
                )
             )
        }
    })
}
