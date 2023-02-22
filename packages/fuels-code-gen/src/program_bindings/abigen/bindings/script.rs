use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abi_types::{FullProgramABI, FullTypeDeclaration},
        abigen::{
            bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
            configurables::generate_code_for_configurable_constatnts,
            logs::logs_lookup_instantiation_code,
        },
        generated_code::GeneratedCode,
    },
    utils::{ident, TypePath},
};

pub(crate) fn script_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let main_function = expand_fn(&abi, shared_types)?;

    let log_type_lookup = logs_lookup_instantiation_code(None, &abi.logged_types, shared_types);

    let configuration_struct_name = ident(&format!("{name}Configurables"));
    let constant_configuration_code = generate_code_for_configurable_constatnts(
        &configuration_struct_name,
        &abi.configurables,
        shared_types,
    )?;

    let code = quote! {
        #[derive(Debug)]
        pub struct #name{
            wallet: ::fuels::signers::wallet::WalletUnlocked,
            binary: ::std::vec::Vec<u8>,
            log_decoder: ::fuels::programs::logs::LogDecoder
        }

        impl #name {
            pub fn new(wallet: ::fuels::signers::wallet::WalletUnlocked, binary_filepath: &str) -> Self {
                let binary = ::std::fs::read(binary_filepath)
                                            .expect("Could not read from binary filepath");
                Self {
                    wallet,
                    binary,
                    log_decoder: ::fuels::programs::logs::LogDecoder {type_lookup: #log_type_lookup}
                }
            }

            pub fn with_configurables(mut self, configurables: ::fuels::programs::Configurables) -> Self {
                configurables.update_constants_in(&mut self.binary);
                self
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

    Ok(GeneratedCode {
        code,
        usable_types: type_paths,
    })
}

fn expand_fn(
    abi: &FullProgramABI,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let encoded_args = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode script arguments");
            let provider = self.wallet.get_provider().expect("Provider not set up").clone();

            ::fuels::programs::script_calls::ScriptCallHandler::new(
                self.binary.clone(),
                encoded_args,
                self.wallet.clone(),
                provider,
                self.log_decoder.clone()
            )
    };

    let original_output_type = generator.output_type();

    generator
        .set_output_type(
            quote! {::fuels::programs::script_calls::ScriptCallHandler<#original_output_type> },
        )
        .set_doc("Run the script's `main` function with the provided arguments".to_string())
        .set_body(body);

    Ok(generator.into())
}
