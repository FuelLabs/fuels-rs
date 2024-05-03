use std::{collections::HashSet, path::PathBuf};

pub use abigen_target::{Abi, AbigenTarget, ProgramType};
use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use inflector::Inflector;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;

use crate::{
    error::Result,
    program_bindings::{
        abigen::bindings::generate_bindings, custom_types::generate_types,
        generated_code::GeneratedCode,
    },
    utils::ident,
};

mod abigen_target;
mod bindings;
mod configurables;
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
        let generated_code = Self::generate_code(no_std, targets)?;

        let use_statements = generated_code.use_statements_for_uniquely_named_types();

        let code = if no_std {
            Self::wasm_paths_hotfix(&generated_code.code())
        } else {
            generated_code.code()
        };

        Ok(quote! {
            #code
            #use_statements
        })
    }
    fn wasm_paths_hotfix(code: &TokenStream) -> TokenStream {
        [
            (r"::\s*std\s*::\s*string", "::alloc::string"),
            (r"::\s*std\s*::\s*format", "::alloc::format"),
            (r"::\s*std\s*::\s*vec", "::alloc::vec"),
            (r"::\s*std\s*::\s*boxed", "::alloc::boxed"),
        ]
        .map(|(reg_expr_str, substitute)| (Regex::new(reg_expr_str).unwrap(), substitute))
        .into_iter()
        .fold(code.to_string(), |code, (regex, wasm_include)| {
            regex.replace_all(&code, wasm_include).to_string()
        })
        .parse()
        .expect("Wasm hotfix failed!")
    }

    fn generate_code(no_std: bool, parsed_targets: Vec<AbigenTarget>) -> Result<GeneratedCode> {
        let custom_types = Self::filter_custom_types(&parsed_targets);
        let shared_types = Self::filter_shared_types(custom_types);

        let bindings = Self::generate_all_bindings(parsed_targets, no_std, &shared_types)?;
        let shared_types = Self::generate_shared_types(shared_types, no_std)?;

        let mod_name = ident("abigen_bindings");
        Ok(shared_types.merge(bindings).wrap_in_mod(mod_name))
    }

    fn generate_all_bindings(
        targets: Vec<AbigenTarget>,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode> {
        targets
            .into_iter()
            .map(|target| Self::generate_binding(target, no_std, shared_types))
            .fold_ok(GeneratedCode::default(), |acc, generated_code| {
                acc.merge(generated_code)
            })
    }

    fn generate_binding(
        target: AbigenTarget,
        no_std: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<GeneratedCode> {
        let mod_name = ident(&format!("{}_mod", &target.name.to_snake_case()));

        let recompile_trigger =
            Self::generate_macro_recompile_trigger(target.source.path.as_ref(), no_std);
        let types = generate_types(&target.source.abi.types, shared_types, no_std)?;
        let bindings = generate_bindings(target, no_std)?;
        Ok(recompile_trigger
            .merge(types)
            .merge(bindings)
            .wrap_in_mod(mod_name))
    }

    /// Any changes to the file pointed to by `path` will cause the reevaluation of the current
    /// procedural macro. This is a hack until <https://github.com/rust-lang/rust/issues/99515>
    /// lands.
    fn generate_macro_recompile_trigger(path: Option<&PathBuf>, no_std: bool) -> GeneratedCode {
        let code = path
            .as_ref()
            .map(|path| {
                let stringified_path = path.display().to_string();
                quote! {
                    const _: &[u8] = include_bytes!(#stringified_path);
                }
            })
            .unwrap_or_default();
        GeneratedCode::new(code, Default::default(), no_std)
    }

    fn generate_shared_types(
        shared_types: HashSet<FullTypeDeclaration>,
        no_std: bool,
    ) -> Result<GeneratedCode> {
        let types = generate_types(&shared_types, &HashSet::default(), no_std)?;

        if types.is_empty() {
            Ok(Default::default())
        } else {
            let mod_name = ident("shared_types");
            Ok(types.wrap_in_mod(mod_name))
        }
    }

    fn filter_custom_types(
        all_types: &[AbigenTarget],
    ) -> impl Iterator<Item = &FullTypeDeclaration> {
        all_types
            .iter()
            .flat_map(|target| &target.source.abi.types)
            .filter(|ttype| ttype.is_custom_type())
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
