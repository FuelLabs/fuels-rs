use std::collections::HashSet;

use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;

use fuels_types::errors::Error;
pub use utils::{AbigenTarget, ProgramType};

use crate::code_gen::abi_types::FullTypeDeclaration;
use crate::code_gen::abigen::contract::Contract;
use crate::code_gen::abigen::predicate::Predicate;
use crate::code_gen::abigen::script::Script;
use crate::code_gen::abigen::utils::{limited_std_prelude, ParsedAbigenTarget};
use crate::code_gen::custom_types;
use crate::code_gen::generated_code::GeneratedCode;
use crate::utils::ident;

mod contract;
mod function_generator;
mod logs;
mod predicate;
mod script;
mod utils;

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
        .fold(GeneratedCode::default(), |all_code, code_segment| {
            all_code.append(code_segment)
        }))
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
                ProgramType::Predicate => {
                    Predicate::generate(&target.name, target.source, no_std, shared_types)
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
        let types = custom_types::generate_types(shared_types.clone(), &HashSet::default())?;

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
}
