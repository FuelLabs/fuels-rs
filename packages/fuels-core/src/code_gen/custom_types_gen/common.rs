use crate::code_gen::resolved_type::{resolve_type, ResolvedType};
use crate::utils::ident;
use anyhow::anyhow;
use fuels_types::errors::Error;
use fuels_types::{TypeApplication, TypeDeclaration};
use inflector::Inflector;
use itertools::Itertools;
use lazy_static::lazy_static;
use proc_macro2::{Ident, LexError, TokenStream};
use quote::quote;
use regex::Regex;
use std::collections::HashMap;

pub struct Component {
    pub field_name: Ident,
    pub field_type: ResolvedType,
}

impl Component {
    pub fn new(
        component: &TypeApplication,
        types: &HashMap<usize, TypeDeclaration>,
        snake_case: bool,
    ) -> anyhow::Result<Component> {
        let field_name = if snake_case {
            component.name.to_snake_case()
        } else {
            component.name.to_owned()
        };

        Ok(Component {
            field_name: ident(&field_name),
            field_type: resolve_type(component, types)?,
        })
    }
}

pub fn impl_try_from(ident: &Ident, generics: &[TokenStream]) -> TokenStream {
    quote! {
        impl<#(#generics: Tokenizable + Parameterize,)*> TryFrom<&[u8]> for #ident<#(#generics,)*> {
            type Error = SDKError;

            fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
                try_from_bytes(bytes)
            }
        }
        impl<#(#generics: Tokenizable + Parameterize,)*> TryFrom<&Vec<u8>> for #ident<#(#generics,)*> {
            type Error = SDKError;

            fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(&bytes)
            }
        }

        impl<#(#generics: Tokenizable + Parameterize,)*> TryFrom<Vec<u8>> for #ident<#(#generics,)*> {
            type Error = SDKError;

            fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(&bytes)
            }
        }
    }
}

pub fn extract_components(
    type_decl: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
    snake_case: bool,
) -> anyhow::Result<Vec<Component>> {
    let components = match &type_decl.components {
        Some(components) if !components.is_empty() => Ok(components),
        _ => Err(anyhow!(
            "Custom type {} must have at least one component!",
            type_decl.type_field
        )),
    }?;

    components
        .iter()
        .map(|component| Component::new(component, types, snake_case))
        .collect()
}

pub fn extract_generic_parameters(field_types: &[Component]) -> Result<Vec<TokenStream>, LexError> {
    field_types
        .iter()
        .map(|Component { field_type, .. }| field_type.get_used_generic_type_names())
        .flatten()
        .unique()
        .map(|arg| arg.parse())
        .collect()
}

// A custom type name should be passed to this function as `{struct,enum} $name`,
pub fn extract_custom_type_name_from_abi_property(prop: &TypeDeclaration) -> Result<Ident, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?:struct|enum)\s*(.*)").unwrap();
    }

    RE.captures(&prop.type_field)
        .map(|captures| ident(&captures[1]))
        .ok_or_else(|| {
            Error::InvalidData(
                "The declared type was not in the format `(enum|struct) name`".to_string(),
            )
        })
}

pub(crate) fn param_type_calls(field_entries: &[Component]) -> Vec<TokenStream> {
    field_entries
        .iter()
        .map(|Component { field_type, .. }| {
            let type_name = &field_type.type_name;
            let parameters = field_type
                .generic_params
                .iter()
                .cloned()
                .map(TokenStream::from)
                .collect::<Vec<_>>();
            if parameters.is_empty() {
                quote! { <#type_name>::param_type() }
            } else {
                quote! { #type_name::<#(#parameters,)*>::param_type() }
            }
        })
        .collect()
}
