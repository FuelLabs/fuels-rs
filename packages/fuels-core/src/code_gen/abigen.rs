use std::collections::{HashMap, HashSet};

use crate::code_gen::bindings::ContractBindings;
use crate::code_gen::full_abi_types::{FullABIFunction, FullLoggedType, FullTypeDeclaration};
use crate::source::Source;
use crate::utils::ident;
use crate::{try_from_bytes, Parameterize, Tokenizable};
use fuel_tx::Receipt;
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use fuels_types::utils::custom_type_name;
use fuels_types::{ProgramABI, ResolvedLog};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use super::custom_types::{expand_custom_enum, expand_custom_struct, single_param_type_call};
use super::functions_gen::expand_function;
use super::resolved_type::resolve_type;

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
    pub contract_name: Ident,
    pub types: Vec<FullTypeDeclaration>,
    pub functions: Vec<FullABIFunction>,
    pub logged_types: Vec<FullLoggedType>,
}

fn limited_std_prelude() -> TokenStream {
    quote! {
            use ::std::clone::Clone;
            use ::std::iter::IntoIterator;
            use ::std::panic;
            use ::std::iter::Iterator;
            use ::std::format;
            use ::std::convert::{Into, TryFrom};
            use ::std::vec;
            use ::std::marker::Sized;
    }
}

fn generate_custom_types(
    types: &[FullTypeDeclaration],
    common_types: &[FullTypeDeclaration],
) -> Result<TokenStream, Error> {
    types
        .iter()
        .filter(|ttype| !Abigen::should_skip_codegen(&ttype.type_field))
        .filter(|ttype| !common_types.contains(ttype))
        .unique()
        .filter_map(|ttype| {
            if ttype.is_struct_type() {
                Some(expand_custom_struct(ttype, common_types))
            } else if ttype.is_enum_type() {
                Some(expand_custom_enum(ttype, common_types))
            } else {
                None
            }
        })
        .fold_ok(TokenStream::default(), |mut acc, stream| {
            acc.extend(stream);
            acc
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
            self.contract_name.to_string().to_lowercase()
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
        common_types: &[FullTypeDeclaration],
    ) -> Result<TokenStream, Error> {
        let name = &self.contract_name;
        let methods_name = ident(&format!("{}Methods", name));

        let contract_functions = self.functions(common_types)?;

        let resolved_logs = self.resolve_logs(common_types);
        let log_id_param_type_pairs = generate_log_id_param_type_pairs(&resolved_logs);
        let fetch_logs = generate_fetch_logs(&resolved_logs);

        let code = if no_std {
            quote! {}
        } else {
            quote! {
                pub struct #name {
                    contract_id: ::fuels::types::bech32::Bech32ContractId,
                    wallet: ::fuels::signers::wallet::WalletUnlocked,
                    logs_lookup: ::std::vec::Vec<(u64, ::fuels::types::param_types::ParamType)>,
                }

                impl #name {
                    pub fn new(contract_id: ::fuels::types::bech32::Bech32ContractId, wallet: ::fuels::signers::wallet::WalletUnlocked) -> Self {
                        Self { contract_id, wallet, logs_lookup: vec![#(#log_id_param_type_pairs),*]}
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

                       ::std::result::Result::Ok(Self { contract_id: self.contract_id.clone(), wallet: wallet, logs_lookup: self.logs_lookup.clone() })
                    }

                    pub async fn get_balances(&self) -> ::std::result::Result<::std::collections::HashMap<::std::string::String, u64>, ::fuels::types::errors::Error> {
                        self.wallet.get_provider()?.get_contract_balances(&self.contract_id).await.map_err(Into::into)
                    }

                    pub fn logs_with_type<D: ::fuels::core::Tokenizable + ::fuels::core::Parameterize>(&self, receipts: &[::fuels::tx::Receipt]) -> ::std::result::Result<::std::vec::Vec<D>, ::fuels::types::errors::Error> {
                        ::fuels::core::code_gen::extract_and_parse_logs(&self.logs_lookup, receipts)
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
                    contract_id: ::fuels::types::bech32::Bech32ContractId,
                    wallet: ::fuels::signers::wallet::WalletUnlocked
                }

                impl #methods_name {
                    #contract_functions
                }
            }
        };

        let custom_types = generate_custom_types(&self.types, common_types)?;
        let prelude = limited_std_prelude();

        let name_mod = self.mod_name();
        Ok(quote! {
            #[allow(clippy::too_many_arguments)]
            #[no_implicit_prelude]
            pub mod #name_mod {
                #prelude

                #custom_types

                #code
            }

            pub use #name_mod::{#name, #methods_name};
        })
    }

    fn functions(&self, common_types: &[FullTypeDeclaration]) -> Result<TokenStream, Error> {
        let tokenized_functions = self
            .functions
            .iter()
            .map(|fun| expand_function(fun, common_types))
            .collect::<Result<Vec<TokenStream>, Error>>()?;

        Ok(quote! { #( #tokenized_functions )* })
    }

    /// Reads the parsed logged types from the ABI and creates ResolvedLogs
    fn resolve_logs(&self, common_types: &[FullTypeDeclaration]) -> Vec<ResolvedLog> {
        self.logged_types
            .iter()
            .map(|l| {
                let resolved_type =
                    resolve_type(&l.application, common_types).expect("Failed to resolve log type");
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

pub struct AbigenContract<'a> {
    pub name: &'a str,
    pub abi_source: &'a str,
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
        let common_types = self.determine_common_types();

        let code = Self::generate_common_types(&common_types)?;

        let code = self
            .contracts
            .iter()
            .map(|contract| contract.expand(self.no_std, &common_types))
            .fold_ok(code, |mut acc, contract_stream| {
                acc.extend(contract_stream);
                acc
            })?;

        Ok(self
            .contracts
            .iter()
            .flat_map(|contract| {
                std::iter::repeat_with(|| contract.mod_name()).zip(&contract.types)
            })
            .filter(|(_, ttype)| {
                let should_use = (ttype.is_enum_type() || ttype.is_struct_type())
                    && !Abigen::should_skip_codegen(&ttype.type_field);
                eprintln!("Should use type {}: {should_use}", ttype.type_field);
                should_use
            })
            .sorted_by(|(_, lhs_ttype), (_, rhs_ttype)| {
                rhs_ttype.type_field.cmp(&lhs_ttype.type_field)
            })
            .group_by(|&(_, ttype)| &ttype.type_field)
            .into_iter()
            .filter_map(|(ttype, group)| {
                let collected = group.collect::<Vec<_>>();
                if collected.len() == 1 {
                    Some((collected.into_iter().next().unwrap().0, ttype))
                } else {
                    None
                }
            })
            .map(|(mod_name, type_field)| {
                let custom_name: TokenStream =
                    custom_type_name(&type_field).unwrap().parse().unwrap();
                quote! {
                    use #mod_name::#custom_name;
                }
            })
            .fold(code, |mut acc, a| {
                acc.extend(a);
                acc
            }))
    }

    fn generate_common_types(common_types: &[FullTypeDeclaration]) -> Result<TokenStream, Error> {
        if common_types.is_empty() {
            return Ok(Default::default());
        }

        let tokenized_common_types = common_types
            .iter()
            .filter(|ttype| !Abigen::should_skip_codegen(&ttype.type_field))
            .unique()
            .filter_map(|ttype| {
                if ttype.is_struct_type() {
                    Some(expand_custom_struct(ttype, common_types))
                } else if ttype.is_enum_type() {
                    Some(expand_custom_enum(ttype, common_types))
                } else {
                    None
                }
            })
            .fold_ok(TokenStream::default(), |mut acc, stream| {
                acc.extend(stream);
                acc
            })?;

        let prelude = limited_std_prelude();
        Ok(quote! {
            #[no_implicit_prelude]
            pub mod shared_types {
                #prelude

                #tokenized_common_types
            }
            pub use shared_types::*;
        })
    }

    fn determine_common_types(&self) -> Vec<FullTypeDeclaration> {
        self.contracts
            .iter()
            .flat_map(|contract| &contract.types)
            .filter(|ttype| ttype.is_enum_type() || ttype.is_struct_type())
            .sorted()
            .group_by(|&el| el)
            .into_iter()
            .filter_map(|(common_type, group)| (group.count() > 1).then_some(common_type))
            .cloned()
            .collect::<Vec<_>>()
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
        ]
        .into_iter()
        .any(|e| e == name)
    }
}

pub fn generate_fetch_logs(resolved_logs: &[ResolvedLog]) -> TokenStream {
    let generate_method = |body: TokenStream| {
        quote! {
            pub fn fetch_logs(&self, receipts: &[::fuels::tx::Receipt]) -> ::std::vec::Vec<::std::string::String> {
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
        let id_to_param_type: ::std::collections::HashMap<_, _> = self.logs_lookup
            .iter()
            .map(|(id, param_type)| (id, param_type))
            .collect();
        let ids_with_data = ::fuels::core::code_gen::extract_log_ids_and_data(receipts);

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
                        ::fuels::core::try_from_bytes::<#type_name>(&data).expect("Failed to construct type from log data.")
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
