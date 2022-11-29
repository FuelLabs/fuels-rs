use super::{
    custom_types::{expand_custom_enum, expand_custom_struct, single_param_type_call},
    functions_gen::expand_function,
    resolved_type::resolve_type,
};
use crate::{
    code_gen::{
        bindings::ContractBindings,
        full_abi_types::{FullABIFunction, FullLoggedType, FullTypeDeclaration},
    },
    source::Source,
    utils::ident,
};
use fuels_types::{
    bech32::Bech32ContractId, errors::Error, param_types::ParamType, utils::custom_type_name,
    ProgramABI, ResolvedLog,
};
use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

pub struct Abigen {
    /// Format the code using a locally installed copy of `rustfmt`.
    rustfmt: bool,
    /// Generate no-std safe code
    no_std: bool,

    contracts: Vec<Contract>,
}

#[derive(Debug)]
pub struct Contract {
    /// The contract name as an identifier.
    contract_name: Ident,
    types: HashSet<FullTypeDeclaration>,
    functions: Vec<FullABIFunction>,
    logged_types: Vec<FullLoggedType>,
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

#[derive(Default)]
pub struct GeneratedCode {
    pub code: TokenStream,
    pub type_paths: HashSet<TypePath>,
}

impl GeneratedCode {
    pub fn merge(mut self, another: GeneratedCode) -> Self {
        self.code.extend(another.code);
        self.type_paths.extend(another.type_paths);
        self
    }

    pub fn prepend_mod_name_to_types(self, mod_name: &Ident) -> Self {
        let path = TypePath::new(&mod_name).unwrap();
        let type_names = self
            .type_paths
            .into_iter()
            .map(|type_path| type_path.prepend(path.clone()))
            .collect();

        Self {
            type_paths: type_names,
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
    // TODO: What if should_skip_abigen skips all types? Then the shared module
    // will still be created bit it will contain nothing. It should not break
    // the code, but we could have lived without it.
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
        .fold_ok(Default::default(), |acc, generated_code| {
            acc.merge(generated_code)
        })
}

impl Contract {
    fn new<S: AsRef<str>>(contract_name: &str, abi_source: S) -> Result<Self, Error> {
        let source = Source::parse(abi_source).expect("failed to parse JSON ABI");

        let json_abi_str = source.get().expect("failed to parse JSON ABI from string");
        let mut parsed_abi: ProgramABI = serde_json::from_str(&json_abi_str)?;

        let types: HashMap<_, _> = parsed_abi
            .types
            .iter()
            .map(|t| (t.type_id, t.clone()))
            .collect();

        let full_types = types
            .values()
            .map(|decl| FullTypeDeclaration::from_counterpart(decl, &types))
            .collect();

        let logged_types = parsed_abi
            .logged_types
            .take()
            .unwrap_or_default()
            .into_iter()
            .map(|l_type| FullLoggedType::from_logged_type(&l_type, &types))
            .collect();

        Ok(Self {
            contract_name: ident(contract_name),
            functions: parsed_abi
                .functions
                .into_iter()
                .map(|fun| FullABIFunction::from_counterpart(&fun, &types))
                .collect(),
            types: full_types,
            logged_types,
        })
    }

    pub fn mod_name(&self) -> Ident {
        ident(&format!(
            "{}_mod",
            self.contract_name.to_string().to_snake_case()
        ))
    }

    /// The high-level goal of this function is to expand* a contract
    /// defined as a JSON into type-safe bindings of that contract that can be
    /// used after it is brought into scope after a successful generation.
    ///
    /// *: To expand, in procedural macro terms, means to automatically generate
    /// Rust code after a transformation of `TokenStream` to another
    /// set of `TokenStream`. This generated Rust code is the brought into scope
    /// after it is called through a procedural macro (`abigen!()` in our case).
    pub fn expand(
        &self,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode, Error> {
        let name_mod = self.mod_name();

        let types_code = generate_types(&self.types, shared_types)?;

        let contract_code = self
            .generate_contract_code(no_std, shared_types)?
            .merge(types_code)
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
                        logs_map: ::fuels::core::code_gen::get_logs_hashmap(&[#(#log_id_param_type_pairs),*], &self.contract_id),
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

impl Abigen {
    /// Creates a new contract with the given ABI JSON source.
    pub fn new<S: AsRef<str>>(contracts: &[(String, S)]) -> Result<Self, Error> {
        let contracts = contracts
            .iter()
            .map(|(name, abi)| Contract::new(name, abi))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            contracts,
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

    pub fn expand(&self) -> Result<TokenStream, Error> {
        let all_code = self.generate_code()?;

        let use_statements = all_code.use_statements_for_uniquely_named_types();
        let code = all_code.code;

        Ok(quote! {
            #code
            #use_statements
        })
    }

    fn generate_code(&self) -> Result<GeneratedCode, Error> {
        let shared_types = self.determine_shared_types();

        let shared_types_code = Self::generate_shared_types(&shared_types)?;

        let contract_code = self
            .contracts
            .iter()
            .map(|contract| contract.expand(self.no_std, &shared_types))
            .fold_ok(GeneratedCode::default(), |acc, generated_code| {
                acc.merge(generated_code)
            })?;

        Ok(shared_types_code.merge(contract_code))
    }

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

        let code = quote! {
            #[no_implicit_prelude]
            pub mod #shared_mod_name {
                #prelude

                #code
            }
        };

        Ok(GeneratedCode { code, type_paths })
    }

    fn determine_shared_types(&self) -> HashSet<FullTypeDeclaration> {
        self.contracts
            .iter()
            .flat_map(|contract| &contract.types)
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

        [
            "ContractId",
            "AssetId",
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
        .any(|e| e == name)
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
    contract_id: &Bech32ContractId,
) -> HashMap<(Bech32ContractId, u64), ParamType> {
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
