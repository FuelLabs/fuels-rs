use std::default::Default;

use fuel_abi_types::abi::full_program::{FullABIFunction, FullProgramABI};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abigen::{
            bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
            configurables::generate_code_for_configurable_constants,
            logs::{generate_id_error_codes_pairs, log_formatters_instantiation_code},
        },
        generated_code::GeneratedCode,
    },
    utils::{TypePath, ident},
};

pub(crate) fn script_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let main_function_abi = extract_main_fn(&abi.functions)?;
    let main_function = expand_fn(main_function_abi)?;

    let log_formatters = log_formatters_instantiation_code(
        quote! {::fuels::types::ContractId::zeroed()},
        &abi.logged_types,
    );

    let error_codes = generate_id_error_codes_pairs(abi.error_codes);
    let error_codes = quote! {vec![#(#error_codes),*].into_iter().collect()};

    let configuration_struct_name = ident(&format!("{name}Configurables"));
    let constant_configuration_code =
        generate_code_for_configurable_constants(&configuration_struct_name, &abi.configurables)?;

    let code = quote! {
        #[derive(Debug,Clone)]
        pub struct #name<A>{
            account: A,
            unconfigured_binary: ::std::vec::Vec<u8>,
            configurables: ::fuels::core::Configurables,
            converted_into_loader: bool,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }

        impl<A> #name<A>
        {
            pub fn new(account: A, binary_filepath: &str) -> Self {
                let binary = ::std::fs::read(binary_filepath)
                                            .expect(&format!("could not read script binary {binary_filepath:?}"));
                Self {
                    account,
                    unconfigured_binary: binary,
                    configurables: ::core::default::Default::default(),
                    converted_into_loader: false,
                    log_decoder: ::fuels::core::codec::LogDecoder::new(#log_formatters, #error_codes),
                    encoder_config: ::fuels::core::codec::EncoderConfig::default(),
                }
            }

            pub fn with_account<U>(self, account: U) -> #name<U> {
                    #name {
                        account,
                        unconfigured_binary: self.unconfigured_binary,
                        log_decoder: self.log_decoder,
                        encoder_config: self.encoder_config,
                        configurables: self.configurables,
                        converted_into_loader: self.converted_into_loader,
                    }
            }

            pub fn with_configurables(mut self, configurables: impl Into<::fuels::core::Configurables>)
                -> Self
            {
                self.configurables = configurables.into();
                self
            }

            pub fn code(&self) -> ::std::vec::Vec<u8> {
                let regular = ::fuels::programs::executable::Executable::from_bytes(self.unconfigured_binary.clone()).with_configurables(self.configurables.clone());

                if self.converted_into_loader {
                    let loader = regular.convert_to_loader().expect("cannot fail since we already converted to the loader successfully");
                    loader.code()
                } else {
                    regular.code()
                }
            }

            pub fn account(&self) -> &A {
                &self.account
            }

            pub fn with_encoder_config(mut self, encoder_config: ::fuels::core::codec::EncoderConfig)
                -> Self
            {
                self.encoder_config = encoder_config;

                self
            }

            pub fn log_decoder(&self) -> ::fuels::core::codec::LogDecoder {
                self.log_decoder.clone()
            }

            /// Will upload the script code as a blob to the network and change the script code
            /// into a loader that will fetch the blob and load it into memory before executing the
            /// code inside. Allows you to optimize fees by paying for most of the code once and
            /// then just running a small loader.
            pub async fn convert_into_loader(&mut self) -> ::fuels::types::errors::Result<&mut Self> where A: ::fuels::accounts::Account + Clone {
                if !self.converted_into_loader {
                    let regular = ::fuels::programs::executable::Executable::from_bytes(self.unconfigured_binary.clone()).with_configurables(self.configurables.clone());
                    let loader = regular.convert_to_loader()?;

                    loader.upload_blob(self.account.clone()).await?;

                    self.converted_into_loader = true;
                }
                ::fuels::types::errors::Result::Ok(self)

            }

        }
        impl<A: ::fuels::accounts::Account + Clone> #name<A> {
            #main_function
        }

        #constant_configuration_code
    };

    // All publicly available types generated above should be listed here.
    let type_paths = [name, &configuration_struct_name]
        .map(|type_name| TypePath::new(type_name).expect("We know the given types are not empty"))
        .into_iter()
        .collect();

    Ok(GeneratedCode::new(code, type_paths, no_std))
}

fn expand_fn(fn_abi: &FullABIFunction) -> Result<TokenStream> {
    let mut generator = FunctionGenerator::new(fn_abi)?;

    let arg_tokens = generator.tokenized_args();
    let original_output_type = generator.output_type();
    let body = quote! {
            let encoded_args = ::fuels::core::codec::ABIEncoder::new(self.encoder_config).encode(&#arg_tokens);

            ::fuels::programs::calls::CallHandler::new_script_call(
                self.code(),
                encoded_args,
                self.account.clone(),
                self.log_decoder.clone()
            )
    };

    generator
        .set_output_type(quote! {::fuels::programs::calls::CallHandler<A, ::fuels::programs::calls::ScriptCall, #original_output_type> })
        .set_docs(fn_abi.doc_strings()?)
        .set_body(body);

    Ok(generator.generate())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::abi::{
        full_program::FullABIFunction,
        program::Attribute,
        unified_program::{UnifiedABIFunction, UnifiedTypeApplication, UnifiedTypeDeclaration},
    };
    use pretty_assertions::assert_eq;
    use quote::quote;

    use crate::{error::Result, program_bindings::abigen::bindings::script::expand_fn};

    #[test]
    fn expand_script_main_function() -> Result<()> {
        let the_function = UnifiedABIFunction {
            inputs: vec![UnifiedTypeApplication {
                name: String::from("bimbam"),
                type_id: 1,
                ..Default::default()
            }],
            name: "main".to_string(),
            attributes: Some(vec![
                Attribute {
                    name: "doc-comment".to_string(),
                    arguments: vec!["This is a doc string".to_string()],
                },
                Attribute {
                    name: "doc-comment".to_string(),
                    arguments: vec!["This is another doc string".to_string()],
                },
            ]),
            ..Default::default()
        };
        let types = [
            (
                0,
                UnifiedTypeDeclaration {
                    type_id: 0,
                    type_field: String::from("()"),
                    ..Default::default()
                },
            ),
            (
                1,
                UnifiedTypeDeclaration {
                    type_id: 1,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = expand_fn(&FullABIFunction::from_counterpart(&the_function, &types)?);

        let expected = quote! {
            #[doc = "This is a doc string"]
            #[doc = "This is another doc string"]
            pub fn main(&self, bimbam: ::core::primitive::bool) -> ::fuels::programs::calls::CallHandler<A, ::fuels::programs::calls::ScriptCall, ()> {
                let encoded_args=::fuels::core::codec::ABIEncoder::new(self.encoder_config)
                    .encode(&[::fuels::core::traits::Tokenizable::into_token(bimbam)]);
                 ::fuels::programs::calls::CallHandler::new_script_call(
                    self.code(),
                    encoded_args,
                    self.account.clone(),
                    self.log_decoder.clone()
                )
            }
        };

        assert_eq!(result?.to_string(), expected.to_string());

        Ok(())
    }
}
