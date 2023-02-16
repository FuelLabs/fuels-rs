use std::collections::HashSet;

use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::{
    error::Result,
    program_bindings::{
        abi_types::{FullTypeApplication, FullTypeDeclaration},
        resolved_type::{resolve_type, ResolvedType},
    },
    utils::{safe_ident, TypePath},
};

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
    ) -> Result<Component> {
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
        .map(|Component { field_type, .. }| single_param_type_call(field_type))
        .collect()
}

/// Returns a TokenStream representing the call to `Parameterize::param_type` for
/// the given ResolvedType. Makes sure to properly handle calls when generics are
/// involved.
pub(crate) fn single_param_type_call(field_type: &ResolvedType) -> TokenStream {
    let type_name = &field_type.type_name;
    let parameters = field_type
        .generic_params
        .iter()
        .map(|resolved_type| resolved_type.to_token_stream())
        .collect::<Vec<_>>();

    if parameters.is_empty() {
        quote! { <#type_name as ::fuels::types::traits::Parameterize>::param_type() }
    } else {
        quote! { <#type_name::<#(#parameters),*> as ::fuels::types::traits::Parameterize>::param_type() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_snake_case_flag() -> Result<()> {
        let type_application = type_application_named("WasNotSnakeCased");

        let sut = Component::new(&type_application, true, &Default::default())?;

        assert_eq!(sut.field_name, "was_not_snake_cased");

        Ok(())
    }

    #[test]
    fn avoids_collisions_with_reserved_keywords() -> Result<()> {
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

pub(crate) fn get_sdk_provided_types() -> Vec<TypePath> {
    [
        "::fuels::types::ContractId",
        "::fuels::types::AssetId",
        "::fuels::types::Address",
        "::fuels::types::Identity",
        "::fuels::types::EvmAddress",
        "::fuels::types::B512",
        "::fuels::types::RawSlice",
        "::std::vec::Vec",
        "::core::result::Result",
        "::core::option::Option",
    ]
    .map(|type_path_str| {
        TypePath::new(type_path_str).expect("known at compile time to be correctly formed")
    })
    .to_vec()
}
