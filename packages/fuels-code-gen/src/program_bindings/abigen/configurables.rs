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
    pub name: String,
    pub ttype: ResolvedType,
    pub offset: u64,
    pub indirect: bool,
}

impl ResolvedConfigurable {
    pub fn new(configurable: &FullConfigurable) -> Result<ResolvedConfigurable> {
        let type_application = &configurable.application;
        Ok(ResolvedConfigurable {
            name: configurable.name.clone(),
            ttype: TypeResolver::default().resolve(type_application)?,
            offset: configurable.offset,
            indirect: configurable.indirect,
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
        #[derive(Clone, Debug)]
        pub struct #configurable_struct_name {
            offsets_with_data: ::std::vec::Vec<(u64, ::std::vec::Vec<u8>)>,
            indirect_configurables: ::std::vec::Vec<u64>,
            encoder: ::fuels::core::codec::ABIEncoder,
        }
    }
}

fn generate_struct_impl(
    configurable_struct_name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let builder_methods = generate_builder_methods(resolved_configurables);
    let indirect_configurables = generate_indirect_configurables(resolved_configurables);

    quote! {
        impl #configurable_struct_name {
            pub fn new(encoder_config: ::fuels::core::codec::EncoderConfig) -> Self {
                Self {
                    encoder: ::fuels::core::codec::ABIEncoder::new(encoder_config),
                    ..::std::default::Default::default()
                }
            }

            #builder_methods
        }

        impl ::std::default::Default for #configurable_struct_name {
            fn default() -> Self {
                Self {
                    offsets_with_data: ::std::default::Default::default(),
                    indirect_configurables: #indirect_configurables,
                    encoder: ::std::default::Default::default(),
                }
            }
        }
    }
}

fn generate_builder_methods(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let methods = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
             ..
         }| {
            let encoder_code = generate_encoder_code(ttype);
            let name = safe_ident(&format!("with_{}", name));
            quote! {
                #[allow(non_snake_case)]
                // Generate the `with_XXX` methods for setting the configurables
                pub fn #name(mut self, value: #ttype) -> ::fuels::prelude::Result<Self> {
                    let encoded = #encoder_code?;
                    self.offsets_with_data.push((#offset, encoded));
                    ::fuels::prelude::Result::Ok(self)
                }
            }
        },
    );

    quote! {
        #(#methods)*
    }
}

fn generate_indirect_configurables(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let indirect_configurables = resolved_configurables.iter().filter_map(
        |ResolvedConfigurable {
             offset, indirect, ..
         }| { indirect.then_some(quote! { #offset }) },
    );

    quote! {
        vec![#(#indirect_configurables),*]
    }
}

fn generate_encoder_code(ttype: &ResolvedType) -> TokenStream {
    quote! {
        self.encoder.encode(&[
                <#ttype as ::fuels::core::traits::Tokenizable>::into_token(value)
            ])
    }
}

fn generate_from_impl(configurable_struct_name: &Ident) -> TokenStream {
    quote! {
        impl From<#configurable_struct_name> for ::fuels::core::Configurables {
            fn from(config: #configurable_struct_name) -> Self {
                ::fuels::core::Configurables::new(config.offsets_with_data, config.indirect_configurables)
            }
        }
    }
}

pub(crate) fn generate_code_for_configurable_reader(
    configurable_struct_name: &Ident,
    configurables: &[FullConfigurable],
) -> Result<TokenStream> {
    let resolved_configurables = configurables
        .iter()
        .map(ResolvedConfigurable::new)
        .collect::<Result<Vec<_>>>()?;

    let struct_decl = generate_struct_decl_reader(configurable_struct_name);
    let struct_impl =
        generate_struct_impl_reader(configurable_struct_name, &resolved_configurables);

    Ok(quote! {
        #struct_decl
        #struct_impl
    })
}

fn generate_struct_decl_reader(configurable_struct_name: &Ident) -> TokenStream {
    quote! {
        #[derive(Clone, Debug)]
        pub struct #configurable_struct_name {
            reader: ::fuels::core::ConfigurablesReader,
        }
    }
}

fn generate_struct_impl_reader(
    configurable_struct_name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let methods = generate_methods_reader(resolved_configurables);

    quote! {
        impl #configurable_struct_name {
            pub fn load_from(
                binary_filepath: impl ::std::convert::AsRef<::std::path::Path>,
            ) -> ::fuels::prelude::Result<Self> {
                let reader = ::fuels::core::ConfigurablesReader::load_from(binary_filepath)?;

                ::fuels::prelude::Result::Ok(Self{reader})
            }

            #methods
        }
    }
}

fn generate_methods_reader(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let methods = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
             indirect,
         }| {
            let name = safe_ident(name);

            let reader_code = if *indirect {
                quote! { self.reader.decode_indirect(#offset as usize) }
            } else {
                quote! { self.reader.decode_direct(#offset as usize) }
            };

            quote! {
                // Generate the `XXX` methods for getting the configurables
                #[allow(non_snake_case)]
                pub fn #name(&self) -> ::fuels::prelude::Result<#ttype> {
                    #reader_code
                }

            }
        },
    );

    quote! {
        #(#methods)*
    }
}
