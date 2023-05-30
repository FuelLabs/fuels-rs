use std::collections::HashMap;

use inflector::Inflector;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::{
    error::Result,
    program_bindings::{
        abi_types::FullTypeApplication,
        resolved_type::{ResolvedType, TypeResolver},
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
        mod_of_component: TypePath,
    ) -> Result<Component> {
        let field_name = if snake_case {
            component.name.to_snake_case()
        } else {
            component.name.to_owned()
        };

        Ok(Component {
            field_name: safe_ident(&field_name),
            field_type: TypeResolver::new(mod_of_component).resolve(component)?,
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
        quote! { <#type_name as ::fuels::core::traits::Parameterize>::param_type() }
    } else {
        quote! { <#type_name::<#(#parameters),*> as ::fuels::core::traits::Parameterize>::param_type() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program_bindings::abi_types::FullTypeDeclaration;

    #[test]
    fn respects_snake_case_flag() -> Result<()> {
        let type_application = type_application_named("WasNotSnakeCased");

        let sut = Component::new(&type_application, true, TypePath::default())?;

        assert_eq!(sut.field_name, "was_not_snake_cased");

        Ok(())
    }

    #[test]
    fn avoids_collisions_with_reserved_keywords() -> Result<()> {
        {
            let type_application = type_application_named("if");

            let sut = Component::new(&type_application, false, TypePath::default())?;

            assert_eq!(sut.field_name, "if_");
        }

        {
            let type_application = type_application_named("let");

            let sut = Component::new(&type_application, false, TypePath::default())?;

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

pub(crate) fn sdk_provided_custom_types_lookup() -> HashMap<TypePath, TypePath> {
    [
        ("std::address::Address", "::fuels::types::Address"),
        ("std::b512::B512", "::fuels::types::B512"),
        ("std::bytes::Bytes", "::fuels::types::Bytes"),
        ("std::contract_id::ContractId", "::fuels::types::ContractId"),
        ("std::identity::Identity", "::fuels::types::Identity"),
        ("std::option::Option", "::core::option::Option"),
        ("std::result::Result", "::core::result::Result"),
        ("std::u128::U128", "u128"),
        ("std::vec::Vec", "::std::vec::Vec"),
        (
            "std::vm::evm::evm_address::EvmAddress",
            "::fuels::types::EvmAddress",
        ),
    ]
    .into_iter()
    .map(|(original_type_path, provided_type_path)| {
        let msg = "known at compile time to be correctly formed";
        (
            TypePath::new(original_type_path).expect(msg),
            TypePath::new(provided_type_path).expect(msg),
        )
    })
    .flat_map(|(original_type_path, provided_type_path)| {
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        let backward_compat_mapping = original_type_path
            .ident()
            .expect("The original type path must have at least one part")
            .into();
        [
            (backward_compat_mapping, provided_type_path.clone()),
            (original_type_path, provided_type_path),
        ]
    })
    .collect()
}

pub(crate) fn get_equivalent_bech32_type(ttype: &str) -> Option<TokenStream> {
    match ttype {
        ":: fuels :: types :: Address" => Some(quote! {::fuels::types::bech32::Bech32Address}),
        ":: fuels :: types :: ContractId" => {
            Some(quote! {::fuels::types::bech32::Bech32ContractId})
        }
        _ => None,
    }
}
