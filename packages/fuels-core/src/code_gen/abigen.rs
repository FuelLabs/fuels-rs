use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use fuel_types::ContractId;
use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use fuels_types::bech32::Bech32ContractId;
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use fuels_types::utils::custom_type_name;
use fuels_types::ResolvedLog;

use crate::code_gen::custom_types::{
    expand_custom_enum, expand_custom_struct, single_param_type_call,
};
use crate::code_gen::full_abi_types::{
    FullABIFunction, FullLoggedType, FullProgramABI, FullTypeDeclaration,
};
use crate::code_gen::functions_gen::expand_function;
use crate::code_gen::resolved_type::resolve_type;
use crate::source::Source;
use crate::utils::ident;

#[derive(Debug)]
pub struct Contract {
    /// The contract name as an identifier.
    contract_name: Ident,
    types: HashSet<FullTypeDeclaration>,
    functions: Vec<FullABIFunction>,
    logged_types: Vec<FullLoggedType>,
}

pub struct Script;

impl Script {
    fn generate(
        contract_name: &str,
        abi: FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name = ident(&self.name);
        let name_mod = ident(&format!("{}_mod", self.name.to_string().to_snake_case()));

        let resolved_logs = self.resolve_logs();
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);

        let main_script_function = self.script_function()?;
        let code = if self.no_std {
            quote! {}
        } else {
            quote! {
                #[derive(Debug)]
                pub struct #name{
                    wallet: WalletUnlocked,
                    binary_filepath: String,
                    logs_map: HashMap<(Bech32ContractId, u64), ParamType>,
                }

                impl #name {
                    pub fn new(wallet: WalletUnlocked, binary_filepath: &str) -> Self {
                        Self {
                            wallet: wallet,
                            binary_filepath: binary_filepath.to_string(),
                            logs_map: get_logs_hashmap(&[#(#log_id_param_type_pairs),*], None)
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

    //
    // pub fn script_function(&self) -> Result<TokenStream, Error> {
    //     let functions = self
    //         .abi
    //         .functions
    //         .iter()
    //         .filter(|function| function.name == "main")
    //         .collect::<Vec<&ABIFunction>>();
    //
    //     if let [main_function] = functions.as_slice() {
    //         let tokenized_function = generate_script_main_function(main_function, &self.types)?;
    //         Ok(quote! { #tokenized_function })
    //     } else {
    //         Err(Error::CompilationError(
    //             "The script must have one function named `main` to compile!".to_string(),
    //         ))
    //     }
    // }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypePath {
    path_parts: Vec<String>,
}

impl TypePath {
    pub fn new<T: ToString>(path: &T) -> Result<Self, Error> {
        let path_str = path.to_string();
        let path_parts = path_str
            .split("::")
            .map(|part| part.to_string())
            .collect::<Vec<_>>();

        if path_parts.is_empty() {
            Err(Error::InvalidType(format!(
                "TypePath cannot be constructed from {path_str} because it's empty!"
            )))
        } else {
            Ok(Self { path_parts })
        }
    }

    pub fn prepend(self, mut another: TypePath) -> Self {
        another.path_parts.extend(self.path_parts);
        another
    }

    pub fn type_name(&self) -> &str {
        self.path_parts
            .last()
            .expect("Must have at least one element")
            .as_str()
    }
}

impl From<&TypePath> for TokenStream {
    fn from(type_path: &TypePath) -> Self {
        let parts = type_path
            .path_parts
            .iter()
            .map(|part| TokenStream::from_str(part).unwrap());
        quote! {
            #(#parts)::*
        }
    }
}
impl From<TypePath> for TokenStream {
    fn from(type_path: TypePath) -> Self {
        (&type_path).into()
    }
}

#[derive(Default)]
pub struct GeneratedCode {
    pub code: TokenStream,
    pub type_paths: HashSet<TypePath>,
}

impl GeneratedCode {
    pub fn append(mut self, another: GeneratedCode) -> Self {
        self.code.extend(another.code);
        self.type_paths.extend(another.type_paths);
        self
    }

    pub fn prepend_mod_name_to_types(self, mod_name: &Ident) -> Self {
        let path = TypePath::new(&mod_name).unwrap();
        let type_paths = self
            .type_paths
            .into_iter()
            .map(|type_path| type_path.prepend(path.clone()))
            .collect();

        Self { type_paths, ..self }
    }

    pub fn use_statements_for_uniquely_named_types(&self) -> TokenStream {
        let type_paths = self
            .types_with_unique_type_name()
            .into_iter()
            .map(TokenStream::from);

        quote! {
            #(pub use #type_paths;)*
        }
    }

    fn types_with_unique_type_name(&self) -> Vec<&TypePath> {
        self.type_paths
            .iter()
            .sorted_by(|&lhs, &rhs| lhs.type_name().cmp(rhs.type_name()))
            .group_by(|&e| e.type_name())
            .into_iter()
            .filter_map(|(_, group)| {
                let mut types = group.collect::<Vec<_>>();
                if types.len() == 1 {
                    Some(types.pop().unwrap())
                } else {
                    None
                }
            })
            .collect()
    }
}

fn limited_std_prelude() -> TokenStream {
    quote! {
            use ::std::{
                clone::Clone,
                convert::{Into, TryFrom},
                format,
                iter::IntoIterator,
                iter::Iterator,
                marker::Sized,
                panic, vec,
                string::ToString
            };
    }
}

fn generate_types(
    types: &HashSet<FullTypeDeclaration>,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    types
        .difference(shared_types)
        .filter(|ttype| !Abigen::should_skip_codegen(&ttype.type_field))
        .filter_map(|ttype| {
            if ttype.is_struct_type() {
                Some(expand_custom_struct(ttype, shared_types))
            } else if ttype.is_enum_type() {
                Some(expand_custom_enum(ttype, shared_types))
            } else {
                None
            }
        })
        .fold_ok(GeneratedCode::default(), |acc, generated_code| {
            acc.append(generated_code)
        })
}

fn parse_program_abi(abi_source: &str) -> Result<FullProgramABI, Error> {
    let source = Source::parse(abi_source).expect("failed to parse JSON ABI");
    let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
    FullProgramABI::from_json_abi(&json_abi_str)
}

impl Contract {
    fn generate(
        contract_name: &str,
        abi: FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        Self {
            contract_name: ident(contract_name),
            functions: abi.functions,
            types: abi.types.into_iter().collect(),
            logged_types: abi.logged_types.into_iter().flatten().collect(),
        }
        .inner_generate(no_std, shared_types)
    }
    fn inner_generate(
        &self,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name_mod = ident(&format!(
            "{}_mod",
            self.contract_name.to_string().to_snake_case()
        ));

        let types_code = generate_types(&self.types, shared_types)?;

        let contract_code = self
            .generate_contract_code(no_std, shared_types)?
            .append(types_code)
            .prepend_mod_name_to_types(&name_mod);

        let code = contract_code.code;
        let prelude = limited_std_prelude();

        let code_wrapped_in_mod = quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod #name_mod {
                #prelude

                #code
            }
        };

        Ok(GeneratedCode {
            code: code_wrapped_in_mod,
            type_paths: contract_code.type_paths,
        })
    }

    fn generate_contract_code(
        &self,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        if !no_std {
            self.generate_std_contract_code(shared_types)
        } else {
            Ok(GeneratedCode::default())
        }
    }

    fn generate_std_contract_code(
        &self,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let resolved_logs = self.resolve_logs(shared_types);
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);

        let methods_name = ident(&format!("{}Methods", &self.contract_name));
        let name = self.contract_name.clone();

        let contract_functions = self.functions(shared_types)?;

        let code = quote! {
            pub struct #name {
                contract_id: ::fuels::types::bech32::Bech32ContractId,
                wallet: ::fuels::signers::wallet::WalletUnlocked,
            }

            impl #name {
                pub fn new(contract_id: ::fuels::types::bech32::Bech32ContractId, wallet: ::fuels::signers::wallet::WalletUnlocked) -> Self {
                    Self { contract_id, wallet }
                }

                pub fn get_contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                    &self.contract_id
                }

                pub fn get_wallet(&self) -> ::fuels::signers::wallet::WalletUnlocked {
                    self.wallet.clone()
                }

                pub fn with_wallet(&self, mut wallet: ::fuels::signers::wallet::WalletUnlocked) -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                   let provider = self.wallet.get_provider()?;
                   wallet.set_provider(provider.clone());

                   ::std::result::Result::Ok(Self { contract_id: self.contract_id.clone(), wallet: wallet })
                }

                pub async fn get_balances(&self) -> ::std::result::Result<::std::collections::HashMap<::std::string::String, u64>, ::fuels::types::errors::Error> {
                    self.wallet.get_provider()?.get_contract_balances(&self.contract_id).await.map_err(Into::into)
                }

                pub fn methods(&self) -> #methods_name {
                    #methods_name {
                        contract_id: self.contract_id.clone(),
                        wallet: self.wallet.clone(),
                        logs_map: ::fuels::core::code_gen::get_logs_hashmap(&[#(#log_id_param_type_pairs),*], ::std::option::Option::Some(self.contract_id.clone())),
                    }
                }
            }

            // Implement struct that holds the contract methods
            pub struct #methods_name {
                contract_id: ::fuels::types::bech32::Bech32ContractId,
                wallet: ::fuels::signers::wallet::WalletUnlocked,
                logs_map: ::std::collections::HashMap<(::fuels::types::bech32::Bech32ContractId, u64), ::fuels::types::param_types::ParamType>,
            }

            impl #methods_name {
                #contract_functions
            }
        };

        let type_paths = [name, methods_name]
            .map(|type_name| {
                TypePath::new(&type_name).expect("We know the given types are not empty")
            })
            .into_iter()
            .collect();

        Ok(GeneratedCode { code, type_paths })
    }

    fn functions(&self, shared_types: &HashSet<FullTypeDeclaration>) -> Result<TokenStream, Error> {
        let tokenized_functions = self
            .functions
            .iter()
            .map(|fun| expand_function(fun, shared_types))
            .collect::<Result<Vec<TokenStream>, Error>>()?;

        Ok(quote! { #( #tokenized_functions )* })
    }
    /// Reads the parsed logged types from the ABI and creates ResolvedLogs
    fn resolve_logs(&self, shared_types: &HashSet<FullTypeDeclaration>) -> Vec<ResolvedLog> {
        self.logged_types
            .iter()
            .map(|l| {
                let resolved_type =
                    resolve_type(&l.application, shared_types).expect("Failed to resolve log type");
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

pub struct AbigenTarget {
    pub name: String,
    pub source: String,
    pub program_type: ProgramType,
}

impl TryFrom<AbigenTarget> for ParsedAbigenTarget {
    type Error = Error;

    fn try_from(value: AbigenTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            source: parse_program_abi(&value.source)?,
            program_type: value.program_type,
        })
    }
}

struct ParsedAbigenTarget {
    pub name: String,
    pub source: FullProgramABI,
    pub program_type: ProgramType,
}

#[derive(Clone, Copy)]
pub enum ProgramType {
    Script,
    Contract,
}

pub struct Abigen;

impl Abigen {
    /// Generate code which can be used to interact with the underlying
    /// contract, script or predicate in a type-safe manner.
    ///
    /// # Arguments
    ///
    /// * `targets`: `AbigenTargets` detailing which ABI to generate bindings
    /// for, and of what nature (Script or Contract).
    /// * `no_std`: don't use the rust std library.
    pub fn generate(targets: Vec<AbigenTarget>, no_std: bool) -> Result<TokenStream, Error> {
        let parsed_targets = Self::parse_targets(targets)?;

        let generated_code = Self::generate_code(no_std, parsed_targets)?;

        let use_statements = generated_code.use_statements_for_uniquely_named_types();
        let code = generated_code.code;

        Ok(quote! {
            #code
            #use_statements
        })
    }

    fn generate_code(
        no_std: bool,
        parsed_targets: Vec<ParsedAbigenTarget>,
    ) -> Result<GeneratedCode, Error> {
        let shared_types = Self::determine_shared_types(&parsed_targets);

        Ok([
            Self::generate_shared_types(&shared_types)?,
            Self::generate_contract_code(no_std, parsed_targets, &shared_types)?,
        ]
        .into_iter()
        .reduce(|all_code, code_segment| all_code.append(code_segment))
        .expect("There is at least one element."))
    }

    fn generate_contract_code(
        no_std: bool,
        parsed_targets: Vec<ParsedAbigenTarget>,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        parsed_targets
            .into_iter()
            .map(|target| match target.program_type {
                ProgramType::Script => {
                    panic!("not yet supported")
                }
                ProgramType::Contract => {
                    Contract::generate(&target.name, target.source, no_std, shared_types)
                }
            })
            .fold_ok(GeneratedCode::default(), |acc, generated_code| {
                acc.append(generated_code)
            })
    }

    fn parse_targets(targets: Vec<AbigenTarget>) -> Result<Vec<ParsedAbigenTarget>, Error> {
        targets
            .into_iter()
            .map(|target| target.try_into())
            .collect()
    }

    ///
    ///
    /// # Arguments
    ///
    /// * `shared_types`: types that appear in multiple contracts, scripts or
    /// predicates.
    ///
    /// returns: Result<GeneratedCode, Error>
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn generate_shared_types(
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        if shared_types.is_empty() {
            return Ok(Default::default());
        }

        let shared_mod_name = ident("shared_types");

        let GeneratedCode { code, type_paths } = generate_types(shared_types, &HashSet::default())?
            .prepend_mod_name_to_types(&shared_mod_name);

        let prelude = limited_std_prelude();

        let code = if code.is_empty() {
            quote! {}
        } else {
            quote! {
                #[no_implicit_prelude]
                pub mod #shared_mod_name {
                    #prelude

                    #code
                }
            }
        };

        Ok(GeneratedCode { code, type_paths })
    }

    fn determine_shared_types(all_types: &[ParsedAbigenTarget]) -> HashSet<FullTypeDeclaration> {
        all_types
            .iter()
            .flat_map(|target| &target.source.types)
            .filter(|ttype| ttype.is_enum_type() || ttype.is_struct_type())
            .sorted()
            .group_by(|&el| el)
            .into_iter()
            .filter_map(|(common_type, group)| (group.count() > 1).then_some(common_type))
            .cloned()
            .collect()
    }

    // Checks whether the given type should not have code generated for it. This
    // is mainly because the corresponding type in Rust already exists --
    // e.g. the contract's Vec type is mapped to std::vec::Vec from the Rust
    // stdlib, ContractId is a custom type implemented by fuels-rs, etc.
    // Others like 'raw untyped ptr' or 'RawVec' are skipped because they are
    // implementation details of the contract's Vec type and are not directly
    // used in the SDK.
    pub fn should_skip_codegen(type_field: &str) -> bool {
        let name = custom_type_name(type_field).unwrap_or_else(|_| type_field.to_string());

        Self::is_type_sdk_provided(&name) || Self::is_type_unused(&name)
    }

    fn is_type_sdk_provided(name: &str) -> bool {
        get_sdk_provided_types()
            .iter()
            .any(|type_path| type_path.type_name() == name)
    }

    fn is_type_unused(name: &str) -> bool {
        ["raw untyped ptr", "RawVec"].contains(&name)
    }
}

pub fn get_sdk_provided_types() -> Vec<TypePath> {
    [
        "::fuels::core::types::ContractId",
        "::fuels::core::types::AssetId",
        "::fuels::core::types::Address",
        "::fuels::core::types::Identity",
        "::fuels::core::types::EvmAddress",
        "::fuels::core::types::B512",
        "::std::vec::Vec",
        "::std::result::Result",
        "::std::option::Option",
    ]
    .map(|type_path_str| {
        TypePath::new(&type_path_str).expect("known at compile time to be correctly formed")
    })
    .to_vec()
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
