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

pub(crate) fn generate_code_for_configurable_constants2(
    configurable_struct_name: &Ident,
    configurables: &[FullConfigurable],
) -> Result<TokenStream> {
    let resolved_configurables = configurables
        .iter()
        .map(ResolvedConfigurable::new)
        .collect::<Result<Vec<_>>>()?;

    let struct_decl = generate_struct_decl2(configurable_struct_name);
    let struct_impl = generate_struct_impl2(configurable_struct_name, &resolved_configurables);

    Ok(quote! {
        #struct_decl
        #struct_impl
    })
}

fn generate_struct_decl2(configurable_struct_name: &Ident) -> TokenStream {
    quote! {
        #[derive(Clone, Debug, Default)]
        pub struct #configurable_struct_name {
            configurables: ::std::collections::HashMap<u64, ::fuels::core::Configurable>,
            encoder: ::fuels::core::codec::ABIEncoder,
            decoder: ::fuels::core::codec::ABIDecoder,
        }
    }
}

fn generate_struct_impl2(
    configurable_struct_name: &Ident,
    resolved_configurables: &[ResolvedConfigurable],
) -> TokenStream {
    let methods = generate_methods2(resolved_configurables);

    let code_to_load_configurables = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             ttype,
             offset,
             indirect,
             ..
         }| {
            let decoder_code = generate_decoder_code2(ttype, *offset as usize);
            quote! {
                let configurable = if #indirect {
                    let offset_from_data_section = Self::extract_usize_at_offset(&binary, #offset as usize)?;
                    ::std::dbg!(&offset_from_data_section);
                    let data_offset =  data_section_offset + offset_from_data_section;

                    let token = self.decoder.decode(&<#ttype as ::fuels::core::traits::Parameterize>::param_type(), &binary[data_offset..])?;
                    let encoded = self.encoder.encode(&[token])?; //TODO: @hal3e remove the encoding

                    ::fuels::core::Configurable::Indirect{
                        offset: #offset,
                        data_offset: data_offset as u64,
                        data: encoded,
                    }
                }
                else {
                    let token = #decoder_code?;
                    let encoded = self.encoder.encode(&[token])?; //TODO: @hal3e remove the encoding
                                                              //and get the num of loaded bytes
                    ::fuels::core::Configurable::Direct{
                        offset: #offset,
                        data: encoded,
                    }
                };

                configurables.insert(#offset, configurable);
            }
        },
    );

    quote! {
        impl #configurable_struct_name {
            pub fn load_from(mut self,
                binary_filepath: impl ::std::convert::AsRef<::std::path::Path>,
            ) -> ::fuels::prelude::Result<Self> {
                let binary_filepath = binary_filepath.as_ref();

                let binary = ::std::fs::read(binary_filepath).map_err(|e| {
                    ::std::io::Error::new(
                        e.kind(),
                        format!("failed to read binary: {binary_filepath:?}: {e}"),
                    )
                })?;

                let mut configurables = ::std::collections::HashMap::new();
                let data_section_offset = Self::extract_usize_at_offset(&binary, 8)?;
                ::std::dbg!(&data_section_offset);


                #(#code_to_load_configurables)*

                self.configurables = configurables;

                ::fuels::prelude::Result::Ok(self)
            }

            fn extract_usize_at_offset(binary: &[u8], offset: usize) -> ::fuels::prelude::Result<usize> {
                if binary.len() < (offset + 8) {
                    return ::std::result::Result::Err(::fuels::types::errors::error!(
                        Other,
                        "given binary is too short to contain a data offset, len: {}",
                        binary.len()
                    ));
                }
                let data_offset =
                    <&[u8] as ::std::convert::TryInto<[u8; 8]>>::try_into(&binary[offset..(offset + 8)]).expect("checked above");


                ::fuels::prelude::Result::Ok(u64::from_be_bytes(data_offset) as usize)
            }

            #methods
        }
    }
}

fn generate_methods2(resolved_configurables: &[ResolvedConfigurable]) -> TokenStream {
    let methods = resolved_configurables.iter().map(
        |ResolvedConfigurable {
             name,
             ttype,
             offset,
             ..
         }| {
            let encoder_code = generate_encoder_code2(ttype);
            let name = safe_ident(name);
            let with_name = safe_ident(&format!("with_{}", name));
            quote! {
                // Generate the `with_XXX` methods for setting the configurables
                #[allow(non_snake_case)]
                pub fn #with_name(mut self, value: #ttype) -> ::fuels::prelude::Result<Self> {
                    let encoded = #encoder_code?;
                    let mut configurable = self.configurables.get(&#offset).expect("is there").clone();
                    configurable.set_data(encoded);
                    self.configurables.insert(#offset, configurable);

                    ::fuels::prelude::Result::Ok(self)
                }

                // Generate the `XXX` methods for getting the configurables
                #[allow(non_snake_case)]
                pub fn #name(&self) -> #ttype {
                    let configurable = self.configurables.get(&#offset).expect("is there");

                    let token = self.decoder.decode(&<#ttype as ::fuels::core::traits::Parameterize>::param_type(), configurable.data()).expect("is ok");

                    <#ttype as ::fuels::core::traits::Tokenizable>::from_token(token).expect("is ok")
                }

            }
        },
    );

    quote! {
        #(#methods)*
    }
}

fn generate_encoder_code2(ttype: &ResolvedType) -> TokenStream {
    quote! {
        self.encoder.encode(&[
                <#ttype as ::fuels::core::traits::Tokenizable>::into_token(value)
            ])
    }
}

fn generate_decoder_code2(ttype: &ResolvedType, offset: usize) -> TokenStream {
    quote! {
        self.decoder.decode(&<#ttype as ::fuels::core::traits::Parameterize>::param_type(), &binary[#offset..])
    }
}
