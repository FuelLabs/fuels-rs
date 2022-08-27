use std::collections::HashMap;

use crate::code_gen::bindings::ContractBindings;
use crate::constants::{ADDRESS_SWAY_NATIVE_TYPE, CONTRACT_ID_SWAY_NATIVE_TYPE};
use crate::source::Source;
use crate::utils::ident;
use fuels_types::errors::Error;
use fuels_types::{ProgramABI, TypeDeclaration};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use super::custom_types_gen::{_new_expand_custom_enum, _new_expand_custom_struct};
use super::functions_gen::_new_expand_function;

// This is a temporary file that duplicates most of the logic in `abigen.rs`.
// This is necessary to support both the old and new JSON ABI formats.
// Once we drop the support for the old one, we can remove this file and rename
// `flat_abigen.rs` to `abigen.rs`.

pub struct FlatAbigen {
    /// Format the code using a locally installed copy of `rustfmt`.
    rustfmt: bool,

    /// Generate no-std safe code
    no_std: bool,

    /// The contract name as an identifier.
    contract_name: Ident,

    abi: ProgramABI,

    types: HashMap<usize, TypeDeclaration>,
}

impl FlatAbigen {
    /// Creates a new contract with the given ABI JSON source.
    pub fn new<S: AsRef<str>>(contract_name: &str, abi_source: S) -> Result<Self, Error> {
        // Support for new JSON ABI file format.
        let source = Source::parse(abi_source).expect("failed to parse JSON ABI");

        let parsed_abi: ProgramABI =
            serde_json::from_str(&source.get().expect("failed to parse JSON ABI from string"))?;

        Ok(Self {
            types: FlatAbigen::get_types(&parsed_abi),
            abi: parsed_abi,
            contract_name: ident(contract_name),
            rustfmt: true,
            no_std: false,
        })
    }

    pub fn no_std(mut self) -> Self {
        self.no_std = true;
        self
    }

    /// Generates the contract bindings.
    pub fn generate(self) -> Result<ContractBindings, Error> {
        let rustfmt = self.rustfmt;
        let tokens = self.expand()?;

        Ok(ContractBindings { tokens, rustfmt })
    }

    /// Entry point of the Abigen's expansion logic.
    /// The high-level goal of this function is to expand* a contract
    /// defined as a JSON into type-safe bindings of that contract that can be
    /// used after it is brought into scope after a successful generation.
    ///
    /// *: To expand, in procedural macro terms, means to automatically generate
    /// Rust code after a transformation of `TokenStream` to another
    /// set of `TokenStream`. This generated Rust code is the brought into scope
    /// after it is called through a procedural macro (`abigen!()` in our case).
    pub fn expand(&self) -> Result<TokenStream, Error> {
        let name = &self.contract_name;
        let builder_name = ident(&format!("{}Builder", name));
        let name_mod = ident(&format!(
            "{}_mod",
            self.contract_name.to_string().to_lowercase()
        ));

        let contract_functions = self.functions()?;
        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;

        let (includes, code) = if self.no_std {
            (
                quote! {
                    use alloc::{vec, vec::Vec};
                    use fuels_core::{EnumSelector, Parameterize, Tokenizable, Token, try_from_bytes};
                    use fuels_types::errors::Error as SDKError;
                    use fuels_types::param_types::{ParamType, EnumVariants};
                },
                quote! {},
            )
        } else {
            (
                quote! {
                    use fuels::contract::contract::{Contract, ContractCallHandler};
                    use fuels::core::{EnumSelector, StringToken, Parameterize, Tokenizable, Token, try_from_bytes};
                    use fuels::signers::LocalWallet;
                    use fuels::tx::{ContractId, Address};
                    use fuels::types::bech32::Bech32ContractId;
                    use fuels::types::errors::Error as SDKError;
                    use fuels::types::param_types::{EnumVariants, ParamType};
                    use std::str::FromStr;
                },
                quote! {
                    pub struct #name {
                        contract_id: Bech32ContractId,
                        wallet: LocalWallet
                    }

                    impl #name {
                        #contract_functions

                        pub fn _get_contract_id(&self) -> &Bech32ContractId {
                            &self.contract_id
                        }

                        pub fn _get_wallet(&self) -> LocalWallet {
                            self.wallet.clone()
                        }

                        pub fn _with_wallet(&self, mut wallet: LocalWallet) -> Result<Self, SDKError> {
                           let provider = self.wallet.get_provider()?;
                           wallet.set_provider(provider.clone());

                           Ok(Self { contract_id: self.contract_id.clone(), wallet: wallet })
                        }
                    }

                    pub struct #builder_name {
                        contract_id: Bech32ContractId,
                        wallet: LocalWallet
                    }

                    impl #builder_name {
                        pub fn new(contract_id: String, wallet: LocalWallet) -> Self {
                            let contract_id = Bech32ContractId::from_str(&contract_id).expect("Invalid contract id");
                            Self { contract_id, wallet }
                        }

                        pub fn contract_id(&mut self, contract_id: String) -> &mut Self {
                            self.contract_id = Bech32ContractId::from_str(&contract_id).expect("Invalid contract id");
                            self
                        }

                        pub fn wallet(&mut self, wallet: LocalWallet) -> &mut Self {
                            self.wallet = wallet;
                            self
                        }

                        pub fn build(self) -> #name {
                            #name { contract_id: self.contract_id, wallet: self.wallet }
                        }
                    }
                },
            )
        };

        Ok(quote! {
            pub use #name_mod::*;

            #[allow(clippy::too_many_arguments)]
            pub mod #name_mod {
                #![allow(clippy::enum_variant_names)]
                #![allow(dead_code)]
                #![allow(unused_imports)]

                #includes

                #code

                #abi_structs
                #abi_enums
            }
        })
    }

    pub fn functions(&self) -> Result<TokenStream, Error> {
        let tokenized_functions = self
            .abi
            .functions
            .iter()
            .map(|function| _new_expand_function(function, &self.types))
            .collect::<Result<Vec<TokenStream>, Error>>()?;

        Ok(quote! { #( #tokenized_functions )* })
    }

    fn abi_structs(&self) -> Result<TokenStream, Error> {
        let mut structs = TokenStream::new();

        // Prevent expanding the same struct more than once
        let mut seen_struct: Vec<&str> = vec![];

        for prop in &self.abi.types {
            // If it isn't a struct, skip.
            if !prop.is_struct_type() {
                continue;
            }

            // Skip custom type generation if the custom type is a Sway-native type.
            // This means ABI methods receiving or returning a Sway-native type
            // can receive or return that native type directly.
            if FlatAbigen::is_sway_native_type(&prop.type_field) {
                continue;
            }

            if !seen_struct.contains(&prop.type_field.as_str()) {
                structs.extend(_new_expand_custom_struct(prop, &self.types)?);
                seen_struct.push(&prop.type_field);
            }
        }

        Ok(structs)
    }

    // Checks whether the given type field is a Sway-native type.
    // It's expected to come in as `"struct T"` or `"enum T"`.
    // `T` is a Sway-native type if it matches exactly one of
    // the reserved strings, such as "Address" or "ContractId".
    fn is_sway_native_type(type_field: &str) -> bool {
        let split: Vec<&str> = type_field.split_whitespace().collect();

        if split.len() > 2 {
            return false;
        }
        split[1] == CONTRACT_ID_SWAY_NATIVE_TYPE || split[1] == ADDRESS_SWAY_NATIVE_TYPE
    }

    fn abi_enums(&self) -> Result<TokenStream, Error> {
        let mut enums = TokenStream::new();

        // Prevent expanding the same enum more than once
        let mut seen_enum: Vec<&str> = vec![];

        for prop in &self.abi.types {
            if !prop.is_enum_type() {
                continue;
            }

            if !seen_enum.contains(&prop.type_field.as_str()) {
                enums.extend(_new_expand_custom_enum(prop, &self.types)?);
                seen_enum.push(&prop.type_field);
            }
        }

        Ok(enums)
    }

    /// Reads the parsed ABI and returns all the types in it.
    pub fn get_types(abi: &ProgramABI) -> HashMap<usize, TypeDeclaration> {
        abi.types.iter().map(|t| (t.type_id, t.clone())).collect()
    }
}

// @todo all (or most, the applicable ones at least) tests in `abigen.rs` should be
// reimplemented for the new JSON ABI format.
// I (@digorithm) skipped writing these tests for now because all this is indirectly
// tested at a higher level in the main harness file. So, I incurred a bit of test debt here.
// Yet, we should test this code directly as well.
