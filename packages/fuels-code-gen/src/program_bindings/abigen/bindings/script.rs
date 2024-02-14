use std::default::Default;

use fuel_abi_types::abi::full_program::FullProgramABI;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abigen::{
            bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
            configurables::generate_code_for_configurable_constants,
            logs::log_formatters_instantiation_code,
        },
        generated_code::GeneratedCode,
    },
    utils::{ident, TypePath},
};

pub(crate) fn script_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let main_function = expand_fn(&abi)?;

    let log_formatters_lookup = log_formatters_instantiation_code(
        quote! {::fuels::types::ContractId::zeroed()},
        &abi.logged_types,
    );

    let configuration_struct_name = ident(&format!("{name}Configurables"));
    let constant_configuration_code =
        generate_code_for_configurable_constants(&configuration_struct_name, &abi.configurables)?;

    let code = quote! {
        #[derive(Debug,Clone)]
        pub struct #name<T: ::fuels::accounts::Account>{
            account: T,
            binary: ::std::vec::Vec<u8>,
            log_decoder: ::fuels::core::codec::LogDecoder,
            encoder_config: ::fuels::core::codec::EncoderConfig,
        }

        impl<T: ::fuels::accounts::Account> #name<T>
        {
            pub fn new(account: T, binary_filepath: &str) -> Self {
                let binary = ::std::fs::read(binary_filepath)
                                            .expect("Could not read from binary filepath");
                Self {
                    account,
                    binary,
                    log_decoder: ::fuels::core::codec::LogDecoder::new(#log_formatters_lookup),
                    encoder_config: ::fuels::core::codec::EncoderConfig::default(),
                }
            }

            pub fn with_account<U: ::fuels::accounts::Account>(self, account: U) -> #name<U> {
                    #name {
                        account,
                        binary: self.binary,
                        log_decoder: self.log_decoder,
                        encoder_config: self.encoder_config,
                    }
            }

            pub fn with_configurables(mut self, configurables: impl Into<::fuels::core::Configurables>)
                -> Self
            {
                let configurables: ::fuels::core::Configurables = configurables.into();
                configurables.update_constants_in(&mut self.binary);
                self
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

fn expand_fn(abi: &FullProgramABI) -> Result<TokenStream> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun)?;

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let encoded_args = ::fuels::core::codec::ABIEncoder::new(self.encoder_config).encode(&#arg_tokens);
            let provider = ::fuels::accounts::ViewOnlyAccount::try_provider(&self.account).expect("Provider not set up")
                .clone();
            ::fuels::programs::script_calls::ScriptCallHandler::new(
                self.binary.clone(),
                encoded_args,
                self.account.clone(),
                provider,
                self.log_decoder.clone()
            )
    };

    let original_output_type = generator.output_type();

    generator
        .set_output_type(
            quote! {::fuels::programs::script_calls::ScriptCallHandler<T, #original_output_type> },
        )
        .set_doc("Run the script's `main` function with the provided arguments".to_string())
        .set_body(body);

    Ok(generator.generate())
}
