use fuel_abi_types::abi::full_program::FullConfigurable;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::resolved_type::{ResolvedType, TypeResolver},
    utils::safe_ident,
};

#[derive(Debug)]
pub(crate) struct ResolvedConfigurable {
    pub name: Ident,
    pub ttype: ResolvedType,
    pub offset: u64,
}

impl ResolvedConfigurable {
    pub fn new(configurable: &FullConfigurable) -> Result<ResolvedConfigurable> {
        let type_application = &configurable.application;
        Ok(ResolvedConfigurable {
            name: safe_ident(&format!("with_{}", configurable.name)),
            ttype: TypeResolver::default().resolve(type_application)?,
            offset: configurable.offset,
        })
    }
}

pub(crate) fn generate_code_for_configurable_constants(
    configurable_struct_name: &Ident,
    configurables: &[FullConfigurable],
) -> Result<TokenStream> {
    let resolved_configurables = configurables
        .iter()
        .map(ResolvedConfigurable::new)
        .collect::<Result<Vec<_>>>()?;

    let struct_decl = generate_struct_decl(configurable_struct_name);
    let struct_impl = generate_struct_impl(configurable_struct_name, &resolved_configurables);
    let from_impl = generate_from_impl(configurable_struct_name);

    Ok(quote! {
        #struct_decl
        #struct_impl
        #from_impl
    })
}

fn generate_struct_decl(configurable_struct_name: &Ident) -> TokenStream {
    quote! {
        #[derive(Clone, Debug, Default)]
        pub struct #configurable_struct_name {
            offsets_with_data: ::std::vec::Vec<(u64, ::std::vec::Vec<u8>)>
        }
    }
}

fn generate_struct_impl(
    configurable_struct_name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let builder_methods = generate_builder_methods(resolved_configurables);

    quote! {
        impl #configurable_struct_name {
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #builder_methods
        }
    }
}

fn generate_builder_methods(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let methods = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
         }| {
            let encoder_code = generate_encoder_code(ttype);
            quote! {
                #[allow(non_snake_case)]
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
        ::fuels::core::codec::ABIEncoder::encode(&[
                <#ttype as ::fuels::core::traits::Tokenizable>::into_token(value)
            ])
            .expect("Cannot encode configurable data")
            .resolve(0)
    }
}

fn generate_from_impl(configurable_struct_name: &Ident) -> TokenStream {
    quote! {
        impl From<#configurable_struct_name> for ::fuels::core::Configurables {
            fn from(config: #configurable_struct_name) -> Self {
                ::fuels::core::Configurables::new(config.offsets_with_data)
            }
        }
    }
}
