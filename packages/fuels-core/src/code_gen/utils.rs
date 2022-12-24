use crate::code_gen::custom_types;
use crate::code_gen::full_abi_types::{FullTypeApplication, FullTypeDeclaration};
use crate::code_gen::resolved_type::{resolve_type, ResolvedType};
use crate::utils::safe_ident;
use fuels_types::errors::Error;
use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use std::collections::HashSet;

// Represents a component of either a struct(field name) or an enum(variant
// name).
#[derive(Debug)]
pub(crate) struct Component {
    pub field_name: Ident,
    pub field_type: ResolvedType,
}

impl Component {
    pub fn new(
        component: &FullTypeApplication,
        snake_case: bool,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<Component, Error> {
        let field_name = if snake_case {
            component.name.to_snake_case()
        } else {
            component.name.to_owned()
        };

        Ok(Component {
            field_name: safe_ident(&field_name),
            field_type: resolve_type(component, shared_types)?,
        })
    }
}

/// Returns TokenStreams representing calls to `Parameterize::param_type` for
/// all given Components. Makes sure to properly handle calls when generics are
/// involved.
pub(crate) fn param_type_calls(field_entries: &[Component]) -> Vec<TokenStream> {
    field_entries
        .iter()
        .map(|Component { field_type, .. }| custom_types::single_param_type_call(field_type))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_snake_case_flag() -> Result<(), Error> {
        let type_application = type_application_named("WasNotSnakeCased");

        let sut = Component::new(&type_application, true, &Default::default())?;

        assert_eq!(sut.field_name, "was_not_snake_cased");

        Ok(())
    }

    #[test]
    fn avoids_collisions_with_reserved_keywords() -> Result<(), Error> {
        {
            let type_application = type_application_named("if");

            let sut = Component::new(&type_application, false, &Default::default())?;

            assert_eq!(sut.field_name, "if_");
        }

        {
            let type_application = type_application_named("let");

            let sut = Component::new(&type_application, false, &Default::default())?;

            assert_eq!(sut.field_name, "let_");
        }

        Ok(())
    }

    fn type_application_named(name: &str) -> FullTypeApplication {
        FullTypeApplication {
            name: name.to_string(),
            type_decl: FullTypeDeclaration {
                type_field: "u64".to_string(),
                components: vec![],
                type_parameters: vec![],
            },
            type_arguments: vec![],
        }
    }
}
