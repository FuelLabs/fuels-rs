use std::collections::HashSet;

pub use abigen_target::{AbigenTarget, ProgramType};
use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abi_types::FullTypeDeclaration,
        abigen::{abigen_target::ParsedAbigenTarget, bindings::generate_bindings},
        custom_types::generate_types,
        generated_code::GeneratedCode,
    },
    utils::ident,
};

mod abigen_target;
mod bindings;
mod logs;

pub struct Abigen;

impl Abigen {
    /// Generate code which can be used to interact with the underlying
    /// contract, script or predicate in a type-safe manner.
    ///
    /// # Arguments
    ///
    /// * `targets`: `AbigenTargets` detailing which ABI to generate bindings
    /// for, and of what nature (Contract, Script or Predicate).
    /// * `no_std`: don't use the Rust std library.
    pub fn generate(targets: Vec<AbigenTarget>, no_std: bool) -> Result<TokenStream> {
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
    ) -> Result<GeneratedCode> {
        let all_custom_types = Self::extract_custom_types(&parsed_targets);
        let shared_types = Self::filter_shared_types(all_custom_types);

        let bindings = Self::generate_all_bindings(parsed_targets, no_std, &shared_types)?;
        let shared_types = Self::generate_shared_types(shared_types)?;

        Ok(shared_types
            .append(bindings)
            .wrap_in_mod(&ident("abigen_bindings")))
    }

    fn generate_all_bindings(
        parsed_targets: Vec<ParsedAbigenTarget>,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode> {
        parsed_targets
            .into_iter()
            .map(|target| Self::generate_binding(target, no_std, shared_types))
            .fold_ok(GeneratedCode::default(), |acc, generated_code| {
                acc.append(generated_code)
            })
    }

    fn generate_binding(
        target: ParsedAbigenTarget,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode> {
        let mod_name = ident(&format!("{}_mod", &target.name.to_snake_case()));

        let types = generate_types(target.source.types.clone(), shared_types)?;
        let bindings = generate_bindings(target, no_std, shared_types)?;

        Ok(limited_std_prelude()
            .append(types)
            .append(bindings)
            .wrap_in_mod(&mod_name))
    }

    fn parse_targets(targets: Vec<AbigenTarget>) -> Result<Vec<ParsedAbigenTarget>> {
        targets
            .into_iter()
            .map(|target| target.try_into())
            .collect()
    }

    fn generate_shared_types(shared_types: HashSet<FullTypeDeclaration>) -> Result<GeneratedCode> {
        let types = generate_types(shared_types, &HashSet::default())?;

        if types.is_empty() {
            Ok(Default::default())
        } else {
            Ok(limited_std_prelude()
                .append(types)
                .wrap_in_mod(&ident("shared_types")))
        }
    }

    fn extract_custom_types(
        all_types: &[ParsedAbigenTarget],
    ) -> impl Iterator<Item = &FullTypeDeclaration> {
        all_types
            .iter()
            .flat_map(|target| &target.source.types)
            .filter(|ttype| ttype.is_enum_type() || ttype.is_struct_type())
    }

    /// A type is considered "shared" if it appears at least twice in
    /// `all_custom_types`.
    ///
    /// # Arguments
    ///
    /// * `all_custom_types`: types from all ABIs whose bindings are being
    /// generated.
    fn filter_shared_types<'a>(
        all_custom_types: impl IntoIterator<Item = &'a FullTypeDeclaration>,
    ) -> HashSet<FullTypeDeclaration> {
        all_custom_types.into_iter().duplicates().cloned().collect()
    }
}

fn limited_std_prelude() -> GeneratedCode {
    let code = quote! {
            use ::std::{
                clone::Clone,
                convert::{Into, TryFrom, From},
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_determines_shared_types() {
        let types = ["type_0", "type_1", "type_0"].map(|type_field| FullTypeDeclaration {
            type_field: type_field.to_string(),
            components: vec![],
            type_parameters: vec![],
        });

        let shared_types = Abigen::filter_shared_types(&types);

        assert_eq!(shared_types, HashSet::from([types[0].clone()]))
    }
}
