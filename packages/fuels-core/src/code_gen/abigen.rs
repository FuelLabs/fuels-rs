use super::{
    custom_types::{expand_custom_enum, expand_custom_struct, single_param_type_call},
    functions_gen::expand_function,
    resolved_type::resolve_type,
};

use crate::{
    code_gen::{
        bindings::ContractBindings,
        functions_gen::{generate_predicate_encode_function, generate_script_main_function},
    },
    source::Source,
    utils::ident,
};
use fuel_tx::ContractId;
use fuels_types::{
    bech32::Bech32ContractId, errors::Error, param_types::ParamType, utils::custom_type_name,
    ABIFunction, ProgramABI, ResolvedLog, TypeDeclaration,
};
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

pub enum AbigenType {
    Contract,
    Script,
    Predicate,
}

pub struct Abigen {
    /// Format the code using a locally installed copy of `rustfmt`.
    rustfmt: bool,
    /// Generate no-std safe code
    no_std: bool,
    /// The contract or script name as an identifier.
    name: String,

    abi: ProgramABI,

    types: HashMap<usize, TypeDeclaration>,

    abigen_type: AbigenType,
}

impl Abigen {
    /// Creates a new contract with the given ABI JSON source.
    pub fn new<S: AsRef<str>>(
        instance_name: &str,
        abi_source: S,
        abigen_type: AbigenType,
    ) -> Result<Self, Error> {
        let source = Source::parse(abi_source).expect("failed to parse JSON ABI");

        let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
        let parsed_abi: ProgramABI = serde_json::from_str(&json_abi_str)?;

        Ok(Self {
            types: Abigen::get_types(&parsed_abi),
            abi: parsed_abi,
            name: instance_name.to_string(),
            rustfmt: true,
            no_std: false,
            abigen_type,
        })
    }

    pub fn no_std(mut self) -> Self {
        self.no_std = true;
        self
    }

    /// Generates the contract bindings.
    pub fn generate(self) -> Result<ContractBindings, Error> {
        let rustfmt = self.rustfmt;
        let tokens = self.expand_contract()?;

        Ok(ContractBindings { tokens, rustfmt })
    }

    pub fn expand(&self) -> Result<TokenStream, Error> {
        match self.abigen_type {
            AbigenType::Contract => self.expand_contract(),
            AbigenType::Script => self.expand_script(),
            AbigenType::Predicate => self.expand_predicate(),
        }
    }

    /// Entry point of the Abigen's expansion logic.
    /// The high-level goal of this function is to expand* a contract defined as a JSON ABI
    /// into type-safe bindings of that contract that can be used after it is brought into
    /// scope after a successful generation.
    ///
    /// *: To expand, in procedural macro terms, means to automatically generate Rust code after a
    /// transformation of `TokenStream` to another set of `TokenStream`. This generated Rust code is
    /// the brought into scope after it is called through a procedural macro
    /// (`abigen!()` in our case).
    pub fn expand_contract(&self) -> Result<TokenStream, Error> {
        let name = ident(&self.name);
        let methods_name = ident(&format!("{}Methods", name));
        let name_mod = ident(&format!("{}_mod", self.name.to_string().to_snake_case()));

        let contract_functions = self.contract_functions()?;
        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;

        let resolved_logs = self.resolve_logs();
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);

        let includes = self.includes();

        let code = if self.no_std {
            quote! {}
        } else {
            quote! {
                pub struct #name {
                 contract_id: Bech32ContractId,
                 wallet: WalletUnlocked,
                 log_decoder: LogDecoder
                }

                impl #name {
                    pub fn new(contract_id: Bech32ContractId, wallet: WalletUnlocked) -> Self {
                        Self {
                            contract_id: contract_id.clone(),
                            wallet,
                            log_decoder: LogDecoder {
                                logs_map: get_logs_hashmap(&[#(#log_id_param_type_pairs),*],
                                                           Some(contract_id))
                            }
                        }
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
                        Ok(Self {
                            contract_id: self.contract_id.clone(),
                            wallet: wallet,
                            log_decoder: self.log_decoder.clone()
                        })
                     }

                    pub async fn get_balances(&self) -> Result<HashMap<String, u64>, SDKError> {
                        self.wallet.get_provider()?.get_contract_balances(&self.contract_id).await.map_err(Into::into)
                    }

                    pub fn methods(&self) -> #methods_name {
                        #methods_name {
                            contract_id: self.contract_id.clone(),
                            wallet: self.wallet.clone(),
                            log_decoder: self.log_decoder.clone()
                        }
                    }
                }

                // Implement struct that holds the contract methods
                pub struct #methods_name {
                    contract_id: Bech32ContractId,
                    wallet: WalletUnlocked,
                    log_decoder: LogDecoder
                }

                impl #methods_name {
                    #contract_functions
                }

                impl SettableContract for #name {
                    fn id(&self) -> Bech32ContractId{
                        self.contract_id.clone()
                    }
                    fn log_decoder(&self) -> LogDecoder{
                        self.log_decoder.clone()
                    }
                }
            }
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

    /// Expand a script into type-safe Rust bindings based on its ABI. See `expand_contract` for
    /// more details.
    pub fn expand_script(&self) -> Result<TokenStream, Error> {
        let name = ident(&self.name);
        let name_mod = ident(&format!("{}_mod", self.name.to_string().to_snake_case()));

        let includes = self.includes();
        let resolved_logs = self.resolve_logs();
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);

        let main_script_function = self.main_function()?;
        let code = if self.no_std {
            quote! {}
        } else {
            quote! {
                #[derive(Debug)]
                pub struct #name{
                    wallet: WalletUnlocked,
                    binary_filepath: String,
                    log_decoder: LogDecoder
                }

                impl #name {
                    pub fn new(wallet: WalletUnlocked, binary_filepath: &str) -> Self {
                        Self {
                            wallet: wallet,
                            binary_filepath: binary_filepath.to_string(),
                            log_decoder: LogDecoder {
                                logs_map: get_logs_hashmap(&[#(#log_id_param_type_pairs),*],
                                                           None)
                            }
                        }
                    }

                    #main_script_function
                }
            }
        };

        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;
        Ok(quote! {
            pub use #name_mod::*;

            #[allow(clippy::too_many_arguments)]
            pub mod #name_mod {
                #![allow(clippy::enum_variant_names)]
                #![allow(dead_code)]

                #includes

                #code

                #abi_structs
                #abi_enums

            }
        })
    }

    /// Expand a predicate into type-safe Rust bindings based on its ABI. See `expand_contract` for
    /// more details.
    pub fn expand_predicate(&self) -> Result<TokenStream, Error> {
        let name = ident(&self.name);
        let name_mod = ident(&format!("{}_mod", self.name.to_string().to_snake_case()));

        let includes = self.includes();

        let encode_data_function = self.main_function()?;
        let code = if self.no_std {
            quote! {}
        } else {
            quote! {
                #[derive(Debug)]
                pub struct #name{
                    address: Bech32Address,
                    code: Vec<u8>,
                    data: UnresolvedBytes
                }

                impl #name {
                    pub fn new(code: Vec<u8>) -> Self {
                        let address: Address = (*Contract::root_from_code(&code)).into();
                        Self {
                            address: address.into(),
                            code,
                            data: UnresolvedBytes::new()
                        }
                    }

                    pub fn load_from(file_path: &str) -> Result<Self, SDKError> {
                        Ok(Self::new(std::fs::read(file_path)?))
                    }

                    pub fn address(&self) -> &Bech32Address {
                        &self.address
                    }

                    pub fn code(&self) -> Vec<u8> {
                        self.code.clone()
                    }

                    pub fn data(&self) -> UnresolvedBytes {
                        self.data.clone()
                    }

                    pub async fn receive(&self, from: &WalletUnlocked, amount:u64, asset_id: AssetId, tx_parameters: Option<TxParameters>) -> Result<(String, Vec<Receipt>), SDKError> {
                        let tx_parameters = tx_parameters.unwrap_or(TxParameters::default());
                        from
                            .transfer(
                                self.address(),
                                amount,
                                asset_id,
                                tx_parameters
                            )
                            .await
                    }

                    pub async fn spend(&self, to: &WalletUnlocked, amount:u64, asset_id: AssetId, tx_parameters: Option<TxParameters>) -> Result<Vec<Receipt>, SDKError> {
                        let tx_parameters = tx_parameters.unwrap_or(TxParameters::default());
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

                    #encode_data_function
                }
            }
        };

        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;
        Ok(quote! {
            pub use #name_mod::*;

            #[allow(clippy::too_many_arguments)]
            pub mod #name_mod {
                #![allow(clippy::enum_variant_names)]
                #![allow(dead_code)]

                #includes

                #code

                #abi_structs
                #abi_enums

            }
        })
    }

    /// Generates the includes necessary for the abigen.
    fn includes(&self) -> TokenStream {
        if self.no_std {
            quote! {
                use alloc::{vec, vec::Vec};
                use fuels_core::{
                    code_gen::function_selector::resolve_fn_selector, try_from_bytes, types::*,
                    EnumSelector, Identity, Parameterize, Token, Tokenizable,
                };
                use fuels_types::{
                    enum_variants::EnumVariants, errors::Error as SDKError,
                    param_types::ParamType,
                };
            }
        } else {
            let specific_includes = match self.abigen_type {
                AbigenType::Contract => quote! {
                    use fuels::contract::contract::{
                        get_decoded_output, Contract, ContractCallHandler,
                    };
                    use fuels::core::{
                        abi_decoder::ABIDecoder, code_gen::function_selector::resolve_fn_selector,
                        EnumSelector, Identity, StringToken,
                    };
                    use fuels::types::ResolvedLog;
                    use std::str::FromStr;
                },
                AbigenType::Script => quote! {
                    use fuels::{
                        contract::script_calls::{ScriptCall, ScriptCallHandler},
                        core::{abi_encoder::ABIEncoder, parameters::TxParameters},
                    };
                    use std::marker::PhantomData;
                },
                AbigenType::Predicate => quote! {
                    use fuels::{
                        core::{abi_encoder::{ABIEncoder, UnresolvedBytes}, parameters::TxParameters},
                        tx::{Contract, AssetId},
                        signers::provider::Provider
                    };
                },
            };
            quote! {
                use fuels::contract::{contract::SettableContract, logs::LogDecoder};
                use fuels::core::{
                    code_gen::get_logs_hashmap, try_from_bytes, types::*, Parameterize, Token,
                    Tokenizable,
                };
                use fuels::signers::WalletUnlocked;
                use fuels::tx::{Address, ContractId, Receipt};
                use fuels::types::{
                    bech32::{Bech32ContractId, Bech32Address}, enum_variants::EnumVariants,
                    errors::Error as SDKError, param_types::ParamType,
                };
                use std::collections::{HashMap, HashSet};
                #specific_includes
            }
        }
    }

    pub fn contract_functions(&self) -> Result<TokenStream, Error> {
        let tokenized_functions = self
            .abi
            .functions
            .iter()
            .map(|function| expand_function(function, &self.types))
            .collect::<Result<Vec<TokenStream>, Error>>()?;
        Ok(quote! { #( #tokenized_functions )* })
    }

    pub fn main_function(&self) -> Result<TokenStream, Error> {
        let functions = self
            .abi
            .functions
            .iter()
            .filter(|function| function.name == "main")
            .collect::<Vec<&ABIFunction>>();

        if let [main_function] = functions.as_slice() {
            let tokenized_function = match self.abigen_type {
                AbigenType::Script => generate_script_main_function(main_function, &self.types),

                AbigenType::Predicate => {
                    generate_predicate_encode_function(main_function, &self.types)
                }
                AbigenType::Contract => Err(Error::CompilationError(
                    "Contract does not have a `main` function!".to_string(),
                )),
            }?;

            Ok(quote! { #tokenized_function })
        } else {
            Err(Error::CompilationError(
                "Only one function named `main` allowed!".to_string(),
            ))
        }
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
            "B512",
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

pub fn get_logs_hashmap(
    id_param_pairs: &[(u64, ParamType)],
    contract_id: Option<Bech32ContractId>,
) -> HashMap<(Bech32ContractId, u64), ParamType> {
    let contract_id = contract_id.unwrap_or_else(|| Bech32ContractId::from(ContractId::zeroed()));
    id_param_pairs
        .iter()
        .map(|(id, param_type)| ((contract_id.clone(), *id), param_type.to_owned()))
        .collect()
}

// @todo all (or most, the applicable ones at least) tests in `abigen.rs` should be
// reimplemented for the new JSON ABI format.
// I (@digorithm) skipped writing these tests for now because all this is indirectly
// tested at a higher level in the main harness file. So, I incurred a bit of test debt here.
// Yet, we should test this code directly as well.
