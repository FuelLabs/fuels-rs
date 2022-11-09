use std::collections::{HashMap, HashSet};

use crate::code_gen::bindings::ContractBindings;
use crate::source::Source;
use crate::utils::ident;
use crate::{try_from_bytes, Parameterize, Tokenizable};
use fuel_tx::Receipt;
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use fuels_types::utils::custom_type_name;
use fuels_types::{ProgramABI, ResolvedLog, TypeDeclaration};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use super::custom_types::{expand_custom_enum, expand_custom_struct, single_param_type_call};
use super::functions_gen::expand_function;
use super::resolved_type::resolve_type;

pub struct Abigen {
    /// Format the code using a locally installed copy of `rustfmt`.
    rustfmt: bool,

    /// Generate no-std safe code
    no_std: bool,

    /// The contract name as an identifier.
    contract_name: Ident,

    abi: ProgramABI,

    types: HashMap<usize, TypeDeclaration>,
}

impl Abigen {
    /// Creates a new contract with the given ABI JSON source.
    pub fn new<S: AsRef<str>>(contract_name: &str, abi_source: S) -> Result<Self, Error> {
        let source = Source::parse(abi_source).expect("failed to parse JSON ABI");

        let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
        let parsed_abi: ProgramABI = serde_json::from_str(&json_abi_str)?;

        Ok(Self {
            types: Abigen::get_types(&parsed_abi),
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
        let methods_name = ident(&format!("{}Methods", name));
        let name_mod = ident(&format!(
            "{}_mod",
            self.contract_name.to_string().to_lowercase()
        ));

        let contract_functions = self.functions()?;
        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;

        let resolved_logs = self.resolve_logs();
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);
        let fetch_logs = generate_fetch_logs(&resolved_logs);

        let (includes, code) = if self.no_std {
            (
                quote! {
                    use alloc::{vec, vec::Vec};
                    use fuels_core::{EnumSelector, Parameterize, Tokenizable, Token, Identity, try_from_bytes};
                    use fuels_core::types::*;
                    use fuels_core::code_gen::function_selector::resolve_fn_selector;
                    use fuels_types::errors::Error as SDKError;
                    use fuels_types::param_types::ParamType;
                    use fuels_types::enum_variants::EnumVariants;
                },
                quote! {},
            )
        } else {
            (
                quote! {
                    use fuels::contract::contract::{Contract, ContractCallHandler};
                     use fuels::core::{EnumSelector, StringToken, Parameterize, Tokenizable, Token,
                                      Identity, try_from_bytes};
                    use fuels::core::code_gen::{extract_and_parse_logs, extract_log_ids_and_data};
                    use fuels::core::abi_decoder::ABIDecoder;
                    use fuels::core::code_gen::function_selector::resolve_fn_selector;
                    use fuels::core::types::*;
                    use fuels::signers::WalletUnlocked;
                    use fuels::tx::{ContractId, Address, Receipt};
                    use fuels::types::bech32::Bech32ContractId;
                    use fuels::types::ResolvedLog;
                    use fuels::types::errors::Error as SDKError;
                    use fuels::types::param_types::ParamType;
                    use fuels::types::enum_variants::EnumVariants;
                    use std::str::FromStr;
                    use std::collections::{HashSet, HashMap};
                },
                quote! {
                    pub struct #name {
                        contract_id: Bech32ContractId,
                        wallet: WalletUnlocked,
                        logs_lookup: Vec<(u64, ParamType)>,
                    }

                    impl #name {
                        pub fn new(contract_id: Bech32ContractId, wallet: WalletUnlocked) -> Self {
                            Self { contract_id, wallet, logs_lookup: vec![#(#log_id_param_type_pairs),*]}
                        }

                        pub fn get_contract_id(&self) -> &Bech32ContractId {
                            &self.contract_id
                        }

                        pub fn get_wallet(&self) -> WalletUnlocked {
                            self.wallet.clone()
                        }

                        pub fn with_wallet(&self, mut wallet: WalletUnlocked) -> Result<Self, SDKError> {
                           let provider = self.wallet.get_provider()?;
                           wallet.set_provider(provider.clone());

                           Ok(Self { contract_id: self.contract_id.clone(), wallet: wallet, logs_lookup: self.logs_lookup.clone() })
                        }

                        pub async fn get_balances(&self) -> Result<HashMap<String, u64>, SDKError> {
                            self.wallet.get_provider()?.get_contract_balances(&self.contract_id).await.map_err(Into::into)
                        }

                        pub fn logs_with_type<D: Tokenizable + Parameterize>(&self, receipts: &[Receipt]) -> Result<Vec<D>, SDKError> {
                            extract_and_parse_logs(&self.logs_lookup, receipts)
                        }

                        #fetch_logs

                        pub fn methods(&self) -> #methods_name {
                            #methods_name {
                                contract_id: self.contract_id.clone(),
                                wallet: self.wallet.clone(),
                            }
                        }
                    }

                    pub struct #methods_name {
                        contract_id: Bech32ContractId,
                        wallet: WalletUnlocked
                    }

                    impl #methods_name {
                        #contract_functions
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
            .map(|function| expand_function(function, &self.types))
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

            if Abigen::should_skip_codegen(&prop.type_field)? {
                continue;
            }

            if !seen_struct.contains(&prop.type_field.as_str()) {
                structs.extend(expand_custom_struct(prop, &self.types)?);
                seen_struct.push(&prop.type_field);
            }
        }

        Ok(structs)
    }

    // Checks whether the given type should not have code generated for it. This
    // is mainly because the corresponding type in Rust already exists --
    // e.g. the contract's Vec type is mapped to std::vec::Vec from the Rust
    // stdlib, ContractId is a custom type implemented by fuels-rs, etc.
    // Others like 'raw untyped ptr' or 'RawVec' are skipped because they are
    // implementation details of the contract's Vec type and are not directly
    // used in the SDK.
    pub fn should_skip_codegen(type_field: &str) -> anyhow::Result<bool> {
        let name = custom_type_name(type_field).unwrap_or_else(|_| type_field.to_string());

        Ok([
            "ContractId",
            "Address",
            "Option",
            "Identity",
            "Result",
            "Vec",
            "raw untyped ptr",
            "RawVec",
            "EvmAddress",
        ]
        .into_iter()
        .any(|e| e == name))
    }

    fn abi_enums(&self) -> Result<TokenStream, Error> {
        let mut enums = TokenStream::new();

        // Prevent expanding the same enum more than once
        let mut seen_enum: Vec<&str> = vec![];

        for prop in &self.abi.types {
            if !prop.is_enum_type() || Abigen::should_skip_codegen(&prop.type_field)? {
                continue;
            }

            if !seen_enum.contains(&prop.type_field.as_str()) {
                enums.extend(expand_custom_enum(prop, &self.types)?);
                seen_enum.push(&prop.type_field);
            }
        }

        Ok(enums)
    }

    /// Reads the parsed ABI and returns all the types in it.
    pub fn get_types(abi: &ProgramABI) -> HashMap<usize, TypeDeclaration> {
        abi.types.iter().map(|t| (t.type_id, t.clone())).collect()
    }

    /// Reads the parsed logged types from the ABI and creates ResolvedLogs
    fn resolve_logs(&self) -> Vec<ResolvedLog> {
        self.abi
            .logged_types
            .as_ref()
            .into_iter()
            .flatten()
            .map(|l| {
                let resolved_type =
                    resolve_type(&l.application, &self.types).expect("Failed to resolve log type");
                let param_type_call = single_param_type_call(&resolved_type);
                let resolved_type_name = TokenStream::from(resolved_type);

                ResolvedLog {
                    log_id: l.log_id,
                    param_type_call,
                    resolved_type_name,
                }
            })
            .collect()
    }
}

pub fn generate_fetch_logs(resolved_logs: &[ResolvedLog]) -> TokenStream {
    let generate_method = |body: TokenStream| {
        quote! {
            pub fn fetch_logs(&self, receipts: &[Receipt]) -> Vec<String> {
                #body
            }
        }
    };

    // if logs are not present, fetch_logs should return an empty string vec
    if resolved_logs.is_empty() {
        return generate_method(quote! { vec![] });
    }

    let branches = generate_param_type_if_branches(resolved_logs);
    let body = quote! {
        let id_to_param_type: HashMap<_, _> = self.logs_lookup
            .iter()
            .map(|(id, param_type)| (id, param_type))
            .collect();
        let ids_with_data = extract_log_ids_and_data(receipts);

        ids_with_data
        .iter()
        .map(|(id, data)|{
            let param_type = id_to_param_type.get(id).expect("Failed to find log id.");

            #(#branches)else*
            else {
                panic!("Failed to parse param type.");
            }
        })
        .collect()
    };

    generate_method(quote! { #body })
}

fn generate_param_type_if_branches(resolved_logs: &[ResolvedLog]) -> Vec<TokenStream> {
    resolved_logs
        .iter()
        .unique_by(|r| r.param_type_call.to_string())
        .map(|r| {
            let type_name = &r.resolved_type_name;
            let param_type_call = &r.param_type_call;

            quote! {
                if **param_type == #param_type_call {
                    return format!(
                        "{:#?}",
                        try_from_bytes::<#type_name>(&data).expect("Failed to construct type from log data.")
                    );
                }
            }
        })
        .collect()
}

fn generate_log_id_param_type_pairs(resolved_logs: &[ResolvedLog]) -> Vec<TokenStream> {
    resolved_logs
        .iter()
        .map(|r| {
            let id = r.log_id;
            let param_type_call = &r.param_type_call;

            quote! {
                (#id, #param_type_call)
            }
        })
        .collect()
}

pub fn extract_and_parse_logs<T: Tokenizable + Parameterize>(
    logs_lookup: &[(u64, ParamType)],
    receipts: &[Receipt],
) -> Result<Vec<T>, Error> {
    let target_param_type = T::param_type();

    let target_ids: HashSet<u64> = logs_lookup
        .iter()
        .filter_map(|(log_id, param_type)| {
            if *param_type == target_param_type {
                Some(*log_id)
            } else {
                None
            }
        })
        .collect();

    let decoded_logs: Vec<T> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::LogData { rb, data, .. } if target_ids.contains(rb) => Some(data.clone()),
            Receipt::Log { ra, rb, .. } if target_ids.contains(rb) => {
                Some(ra.to_be_bytes().to_vec())
            }
            _ => None,
        })
        .map(|data| try_from_bytes(&data))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(decoded_logs)
}

pub fn extract_log_ids_and_data(receipts: &[Receipt]) -> Vec<(u64, Vec<u8>)> {
    receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::LogData { rb, data, .. } => Some((*rb, data.clone())),
            Receipt::Log { ra, rb, .. } => Some((*rb, ra.to_be_bytes().to_vec())),
            _ => None,
        })
        .collect()
}

// @todo all (or most, the applicable ones at least) tests in `abigen.rs` should be
// reimplemented for the new JSON ABI format.
// I (@digorithm) skipped writing these tests for now because all this is indirectly
// tested at a higher level in the main harness file. So, I incurred a bit of test debt here.
// Yet, we should test this code directly as well.
