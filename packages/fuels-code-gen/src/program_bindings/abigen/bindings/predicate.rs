use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abi_types::{FullProgramABI, FullTypeDeclaration},
        abigen::bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
        generated_code::GeneratedCode,
    },
    utils::TypePath,
};

pub(crate) fn predicate_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let encode_function = expand_fn(&abi, shared_types)?;

    let code = quote! {

        #[derive(Debug, Clone)]
        pub struct #name {
            address: ::fuels::types::bech32::Bech32Address,
            code: ::std::vec::Vec<u8>,
            data: ::fuels::core::abi_encoder::UnresolvedBytes,
            provider: ::std::option::Option<::fuels::prelude::Provider>
        }

        impl #name {

            pub fn get_predicate(&self) -> ::fuels::programs::predicate::Predicate {
                ::fuels::programs::predicate::Predicate {
                    address: self.address.clone(),
                    code: self.code.clone(),
                    data: self.data.clone(),
                    provider: self.provider.clone(),
                }
            }

            pub fn new(code: ::std::vec::Vec<u8>) -> Self {
                let address: ::fuels::types::Address = (*::fuels::tx::Contract::root_from_code(&code)).into();
                Self {
                    address: address.clone().into(),
                    code,
                    data: ::fuels::core::abi_encoder::UnresolvedBytes::new(),
                    provider: ::std::option::Option::None
                }
            }

            pub fn load_from(file_path: &str) -> ::fuels::types::errors::Result<Self> {
                ::std::result::Result::Ok(Self::new(::std::fs::read(file_path)?))
            }

           #encode_function
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
) -> Result<TokenStream> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let arg_tokens = generator.tokenized_args();

    let body = quote! {
        let data = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode predicate data");

        Self {
            address: self.address.clone(),
            code: self.code.clone(),
            data,
            provider: self.provider.clone()
        }
    };

    generator
        .set_doc("Run the predicate's encode function with the provided arguments".to_string())
        .set_name("encode_data".to_string())
        .set_output_type(quote! {Self})
        .set_body(body);

    Ok(generator.into())
}
