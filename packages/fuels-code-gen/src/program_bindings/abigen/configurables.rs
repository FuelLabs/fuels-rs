use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abi_types::{FullConfigurable, FullTypeDeclaration},
        resolved_type::{resolve_type, ResolvedType},
    },
    utils::safe_ident,
};

#[derive(Debug)]
pub(crate) struct ResolvedConfigurable {
    pub name: Ident,
    pub ttype: ResolvedType,
    pub offset: u64,
}

impl ResolvedConfigurable {
    pub fn new(
        configurable: &FullConfigurable,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<ResolvedConfigurable> {
        Ok(ResolvedConfigurable {
            name: safe_ident(&format!("set_{}", configurable.name.to_lowercase())),
            ttype: resolve_type(&configurable.application, shared_types)?,
            offset: configurable.offset,
        })
    }
}

pub(crate) fn generate_code_for_configurable_constatnts(
    name: &Ident,
    configurables: &[FullConfigurable],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream> {
    let resolved_configurables = configurables
        .iter()
        .map(|c| ResolvedConfigurable::new(c, shared_types))
        .collect::<Result<Vec<_>>>()?;

    let config_methods = generate_configurable_impl(name, &resolved_configurables);
    let from_impl = generate_from_impl(name);

    Ok(quote! {
        #[derive(Clone, Debug, Default)]
        pub struct #name {
            pub offsets_with_data: ::std::vec::Vec<(u64, ::std::vec::Vec<u8>)>
        }

        #config_methods
        #from_impl
    })
}

fn generate_configurable_impl(
    name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let methods = generate_method_functions(resolved_configurables);

    quote! {
        impl #name {
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #methods
        }
    }
}

fn generate_method_functions(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let methods = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
         }| {
            let encoder_code = generate_encoder_code(ttype);
            quote! {
                pub fn #name(mut self, value: #ttype) -> Self{
                    self.offsets_with_data.push((#offset, #encoder_code));
                    self
                }
            }
        },
    );

    quote! {
        #(#methods)*
    }
}

fn generate_encoder_code(ttype: &ResolvedType) -> TokenStream {
    quote! {
        ::fuels::core::abi_encoder::ABIEncoder::encode(&[
                <#ttype as ::fuels::types::traits::Tokenizable>::into_token(value)
            ])
            .expect("Cannot encode configurable data")
            .resolve(0)
    }
}

fn generate_from_impl(name: &Ident) -> TokenStream {
    quote! {
        impl From<#name> for ::fuels::programs::Configurables {
            fn from(config: #name) -> Self {
                ::fuels::programs::Configurables {
                    offsets_with_data: config.offsets_with_data.clone()
                }
            }
        }
    }
}
