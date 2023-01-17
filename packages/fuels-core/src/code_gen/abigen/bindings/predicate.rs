use std::collections::HashSet;

use fuels_types::errors::Error;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::code_gen::{
    abi_types::{FullProgramABI, FullTypeDeclaration},
    abigen::bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
    generated_code::GeneratedCode,
    type_path::TypePath,
};

pub(crate) fn predicate_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let encode_function = expand_fn(&abi, shared_types)?;

    let code = quote! {
        #[derive(Debug)]
        pub struct #name {
            address: ::fuels::types::bech32::Bech32Address,
            code: ::std::vec::Vec<u8>,
            data: ::fuels::core::abi_encoder::UnresolvedBytes
        }

        impl #name {
            pub fn new(code: ::std::vec::Vec<u8>) -> Self {
                let address: ::fuels::types::Address = (*::fuels::tx::Contract::root_from_code(&code)).into();
                Self {
                    address: address.into(),
                    code,
                    data: ::fuels::core::abi_encoder::UnresolvedBytes::new()
                }
            }

            pub fn load_from(file_path: &str) -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                ::std::result::Result::Ok(Self::new(::std::fs::read(file_path)?))
            }

            pub fn address(&self) -> &::fuels::types::bech32::Bech32Address {
                &self.address
            }

            pub fn code(&self) -> ::std::vec::Vec<u8> {
                self.code.clone()
            }

            pub fn data(&self) -> ::fuels::core::abi_encoder::UnresolvedBytes {
                self.data.clone()
            }

            pub async fn receive(&self, from: &::fuels::signers::wallet::WalletUnlocked,
                                 amount: u64,
                                 asset_id: ::fuels::types::AssetId,
                                 tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>
            ) -> ::std::result::Result<(::std::string::String, ::std::vec::Vec<::fuels::tx::Receipt>), ::fuels::types::errors::Error> {
                let tx_parameters = tx_parameters.unwrap_or_default();
                from
                    .transfer(
                        self.address(),
                        amount,
                        asset_id,
                        tx_parameters
                    )
                    .await
            }

            pub async fn spend(&self, to: &::fuels::signers::wallet::WalletUnlocked,
                                amount: u64,
                                asset_id: ::fuels::types::AssetId,
                                tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>
            ) -> ::std::result::Result<::std::vec::Vec<::fuels::tx::Receipt>, ::fuels::types::errors::Error> {
                let tx_parameters = tx_parameters.unwrap_or_default();
                to
                    .receive_from_predicate(
                        self.address(),
                        self.code(),
                        amount,
                        asset_id,
                        self.data(),
                        tx_parameters,
                    )
                    .await
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
) -> Result<TokenStream, Error> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
        let data = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode predicate data");

        Self {
            address: self.address.clone(),
            code: self.code.clone(),
            data
        }
    };

    generator
        .set_doc("Run the predicate's encode function with the provided arguments".to_string())
        .set_name("encode_data".to_string())
        .set_output_type(quote! {Self})
        .set_body(body);

    Ok(generator.into())
}
