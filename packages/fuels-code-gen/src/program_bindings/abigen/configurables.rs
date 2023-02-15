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
            name: safe_ident(&configurable.name),
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

    let c_struct = generate_sturct_for_configurable_constants(name, &resolved_configurables);
    let from_impl = generate_from_impl(name, &resolved_configurables);

    Ok(quote! {
        #c_struct
        #from_impl
    })
}

fn generate_sturct_for_configurable_constants(
    name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let fields =
        resolved_configurables
            .iter()
            .map(|ResolvedConfigurable { name, ttype, .. }| {
                quote! { pub #name: #ttype }
            });

    quote! {
        #[derive(Clone, Debug)]
        pub struct #name {
            #(#fields),*
        }
    }
}

fn generate_from_impl(
    name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let replacable_configurables = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
         }| {
            let encoder_code = generate_encoder_code(name, ttype);

            quote! {
                (#offset, #encoder_code)
            }
        },
    );

    quote! {
        impl From<#name> for ::fuels::programs::contract::ReplaceConfigurable {
            fn from(configurable: #name) -> Self {
                ::fuels::programs::contract::ReplaceConfigurable {
                    configurables: vec![#(#replacable_configurables),*],
                }
            }
        }
    }
}

fn generate_encoder_code(name: &Ident, ttype: &ResolvedType) -> TokenStream {
    quote! {
        ::fuels::core::abi_encoder::ABIEncoder::encode(&[
                <#ttype as ::fuels::types::traits::Tokenizable>::into_token(configurable.#name)
            ])
            .expect("Cannot encode configurable data")
            .resolve(0)
    }
}
