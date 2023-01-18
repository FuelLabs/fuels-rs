use std::collections::HashSet;

use fuels_types::errors::Error;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::code_gen::{
    abi_types::{FullProgramABI, FullTypeDeclaration},
    abigen::{
        bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
        logs::logs_lookup_instantiation_code,
    },
    generated_code::GeneratedCode,
    type_path::TypePath,
};

pub(crate) fn script_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let main_function = expand_fn(&abi, shared_types)?;

    let log_type_lookup = logs_lookup_instantiation_code(None, &abi.logged_types, shared_types);

    let code = quote! {
        #[derive(Debug)]
        pub struct #name{
            wallet: ::fuels::signers::wallet::WalletUnlocked,
            binary_filepath: ::std::string::String,
            log_decoder: ::fuels::programs::logs::LogDecoder
        }

        impl #name {
            pub fn new(wallet: ::fuels::signers::wallet::WalletUnlocked, binary_filepath: &str) -> Self {
                Self {
                    wallet,
                    binary_filepath: binary_filepath.to_string(),
                    log_decoder: ::fuels::programs::logs::LogDecoder {type_lookup: #log_type_lookup}
                }
            }

            #main_function
        }
    };

    // All publicly available types generated above should be listed here.
    let type_paths = [TypePath::new(name).expect("We know name is not empty.")].into();

    Ok(GeneratedCode {
        code,
        usable_types: type_paths,
    })
}

fn expand_fn(
    abi: &FullProgramABI,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream, Error> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let script_binary = ::std::fs::read(&self.binary_filepath)
                                        .expect("Could not read from binary filepath");
            let encoded_args = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode script arguments");
            let provider = self.wallet.get_provider().expect("Provider not set up").clone();

            ::fuels::programs::script_calls::ScriptCallHandler::new(
                script_binary,
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
