use std::default::Default;
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
use crate::code_gen::functions_gen::{expand_function, generate_script_main_function};
use crate::code_gen::resolved_type::resolve_type;
use crate::source::Source;
use crate::utils::ident;

/// Reads the parsed logged types from the ABI and creates ResolvedLogs
fn resolve_logs(
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Vec<ResolvedLog> {
    logged_types
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

fn logs_hashmap_type() -> TokenStream {
    quote! {::std::collections::HashMap<(::fuels::types::bech32::Bech32ContractId, u64), ::fuels::types::param_types::ParamType>}
}

fn logs_hashmap_instantiation_code(
    contract_id: Option<TokenStream>,
    logged_types: &[FullLoggedType],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> TokenStream {
    let resolved_logs = resolve_logs(&logged_types, shared_types);
    let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);
    let contract_id = contract_id
        .map(|id| quote! { ::std::option::Option::Some(#id) })
        .unwrap_or_else(|| quote! {::std::option::Option::None});
    quote! {::fuels::core::code_gen::get_logs_hashmap(&[#(#log_id_param_type_pairs),*], #contract_id)}
}

#[derive(Debug)]
pub struct Contract;

impl Contract {
    fn generate(
        contract_name: &str,
        abi: FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name_mod = ident(&format!(
            "{}_mod",
            contract_name.to_string().to_snake_case()
        ));

        let types = generate_types(abi.types.clone(), shared_types)?;

        let contract_bindings =
            Self::generate_contract_code(contract_name, &abi, no_std, shared_types)?;

        Ok(limited_std_prelude()
            .append(contract_bindings)
            .append(types)
            .wrap_in_mod(&name_mod))
    }

    fn generate_contract_code(
        contract_name: &str,
        abi: &FullProgramABI,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        if no_std {
            Ok(GeneratedCode::default())
        } else {
            Self::generate_std_contract_code(contract_name, abi, shared_types)
        }
    }

    fn generate_std_contract_code(
        contract_name: &str,
        abi: &FullProgramABI,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let logs_map = logs_hashmap_instantiation_code(
            Some(quote! {self.contract_id.clone()}),
            &abi.logged_types,
            shared_types,
        );
        let logs_map_type = logs_hashmap_type();

        let methods_name = ident(&format!("{}Methods", &contract_name));
        let name = ident(contract_name);

        let contract_functions = Self::functions(&abi.functions, shared_types)?;

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
                        logs_map: #logs_map
                    }
                }
            }

            // Implement struct that holds the contract methods
            pub struct #methods_name {
                contract_id: ::fuels::types::bech32::Bech32ContractId,
                wallet: ::fuels::signers::wallet::WalletUnlocked,
                logs_map: #logs_map_type
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

        Ok(GeneratedCode {
            code,
            usable_types: type_paths,
        })
    }

    fn functions(
        functions: &[FullABIFunction],
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<TokenStream, Error> {
        let tokenized_functions = functions
            .iter()
            .map(|fun| expand_function(fun, shared_types))
            .collect::<Result<Vec<TokenStream>, Error>>()?;

        Ok(quote! { #( #tokenized_functions )* })
    }
}

pub struct Script;

impl Script {
    fn generate(
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
        let functions = abi
            .functions
            .iter()
            .filter(|function| function.name == "main")
            .collect::<Vec<&FullABIFunction>>();

        if let [main_function] = functions.as_slice() {
            let tokenized_function = generate_script_main_function(main_function, shared_types)?;
            Ok(quote! { #tokenized_function })
        } else {
            Err(Error::CompilationError(
                "The script must have one function named `main` to compile!".to_string(),
            ))
        }
    }
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
    pub usable_types: HashSet<TypePath>,
}

impl GeneratedCode {
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    pub fn append(mut self, another: GeneratedCode) -> Self {
        self.code.extend(another.code);
        self.usable_types.extend(another.usable_types);
        self
    }

    pub fn wrap_in_mod(self, mod_name: &Ident) -> Self {
        let mod_path = TypePath::new(&mod_name).unwrap();
        let type_paths = self
            .usable_types
            .into_iter()
            .map(|type_path| type_path.prepend(mod_path.clone()))
            .collect();

        let inner_code = self.code;
        let code = quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod #mod_name {
                #inner_code
            }
        };

        Self {
            code,
            usable_types: type_paths,
        }
    }

    pub fn prepend_mod_name_to_types(self, mod_name: &Ident) -> Self {
        let path = TypePath::new(&mod_name).unwrap();
        let type_paths = self
            .usable_types
            .into_iter()
            .map(|type_path| type_path.prepend(path.clone()))
            .collect();

        Self {
            usable_types: type_paths,
            ..self
        }
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
        self.usable_types
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

fn limited_std_prelude() -> GeneratedCode {
    let code = quote! {
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
    };

    GeneratedCode {
        code,
        ..Default::default()
    }
}

fn generate_types<T: IntoIterator<Item = FullTypeDeclaration>>(
    types: T,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    HashSet::from_iter(types)
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
            Self::generate_bindings(no_std, parsed_targets, &shared_types)?,
        ]
        .into_iter()
        .reduce(|all_code, code_segment| all_code.append(code_segment))
        .expect("There is at least one element."))
    }

    fn generate_bindings(
        no_std: bool,
        parsed_targets: Vec<ParsedAbigenTarget>,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        parsed_targets
            .into_iter()
            .map(|target| match target.program_type {
                ProgramType::Script => {
                    Script::generate(&target.name, target.source, no_std, shared_types)
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
        let types = generate_types(shared_types.clone(), &HashSet::default())?;

        if types.is_empty() {
            Ok(Default::default())
        } else {
            Ok(limited_std_prelude()
                .append(types)
                .wrap_in_mod(&ident("shared_types")))
        }
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
