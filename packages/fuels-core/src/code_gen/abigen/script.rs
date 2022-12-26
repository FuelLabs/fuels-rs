use std::collections::HashSet;

use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;

use fuels_types::errors::Error;

use crate::code_gen::abi_types::{FullABIFunction, FullProgramABI, FullTypeDeclaration};
use crate::code_gen::abigen::function_generator::FunctionGenerator;
use crate::code_gen::abigen::logs::{logs_hashmap_instantiation_code, logs_hashmap_type};
use crate::code_gen::abigen::utils::{extract_main_fn, limited_std_prelude};
use crate::code_gen::custom_types::generate_types;
use crate::code_gen::generated_code::GeneratedCode;
use crate::code_gen::type_path::TypePath;
use crate::utils::ident;

pub(crate) struct Script;

impl Script {
    pub(crate) fn generate(
        script_name: &str,
        abi: FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name_mod = ident(&format!("{}_mod", script_name.to_string().to_snake_case()));

        let types_code = generate_types(abi.types.clone(), shared_types)?;

        let script_code =
            Self::generate_script_code(script_name, &abi, no_std, shared_types)?.append(types_code);

        Ok(limited_std_prelude()
            .append(script_code)
            .wrap_in_mod(&name_mod))
    }

    fn generate_script_code(
        script_name: &str,
        abi: &FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        if no_std {
            Ok(GeneratedCode::default())
        } else {
            Self::generate_std_script_code(script_name, abi, shared_types)
        }
    }

    fn generate_std_script_code(
        script_name: &str,
        abi: &FullProgramABI,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name = ident(script_name);

        let main_function = Self::script_function(abi, shared_types)?;

        let logs_map = logs_hashmap_instantiation_code(None, &abi.logged_types, shared_types);
        let logs_map_type = logs_hashmap_type();

        let code = quote! {
            #[derive(Debug)]
            pub struct #name{
                wallet: ::fuels::signers::wallet::WalletUnlocked,
                binary_filepath: ::std::string::String,
                logs_map: #logs_map_type
            }

            impl #name {
                pub fn new(wallet: ::fuels::signers::wallet::WalletUnlocked, binary_filepath: &str) -> Self {
                    Self {
                        wallet,
                        binary_filepath: binary_filepath.to_string(),
                        logs_map: #logs_map
                    }
                }

                #main_function
            }
        };

        let type_paths = [TypePath::new(&name).expect("We know name is not empty.")].into();

        Ok(GeneratedCode {
            code,
            usable_types: type_paths,
        })
    }

    fn script_function(
        abi: &FullProgramABI,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<TokenStream, Error> {
        extract_main_fn(&abi.functions).and_then(|fun| expand_script_main_fn(fun, shared_types))
    }
}

/// Generate the `main` function of a script
fn expand_script_main_fn(
    fun: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream, Error> {
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let original_output_type = generator.output_type();
    generator
        .set_output_type(
            quote! {::fuels::contract::script_calls::ScriptCallHandler<#original_output_type> },
        )
        .set_doc("Run the script's `main` function with the provided arguments".to_string());

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let script_binary = ::std::fs::read(&self.binary_filepath)
                                        .expect("Could not read from binary filepath");
            let encoded_args = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode script arguments");
            let provider = self.wallet.get_provider().expect("Provider not set up").clone();
            let log_decoder = ::fuels::contract::logs::LogDecoder{logs_map: self.logs_map.clone()};

            ::fuels::contract::script_calls::ScriptCallHandler::new(
                script_binary,
                encoded_args,
                self.wallet.clone(),
                provider,
                log_decoder
            )
    };

    generator.set_body(body);

    Ok(generator.into())
}
