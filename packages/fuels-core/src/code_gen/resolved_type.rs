use crate::utils::{ident, safe_ident};
use fuels_types::errors::Error;
use fuels_types::utils::custom_type_name;
use fuels_types::utils::{
    extract_array_len, extract_generic_name, extract_str_len, has_tuple_format,
};
use fuels_types::{TypeApplication, TypeDeclaration};
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

// Represents a type alongside its generic parameters. Can be converted into a
// `TokenStream` via `.into()`.
#[derive(Debug, Clone)]
pub struct ResolvedType {
    pub type_name: TokenStream,
    pub generic_params: Vec<ResolvedType>,
}

impl ResolvedType {
    pub fn is_unit(&self) -> bool {
        self.type_name.to_string() == "()"
    }
    // Used to prevent returning vectors until we get the compiler support for
    // it.
    #[must_use]
    pub fn uses_vectors(&self) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\bVec\b").unwrap();
        }
        RE.is_match(&self.type_name.to_string())
            || self.generic_params.iter().any(ResolvedType::uses_vectors)
    }
}

impl Display for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", TokenStream::from(self.clone()))
    }
}

impl From<&ResolvedType> for TokenStream {
    fn from(resolved_type: &ResolvedType) -> Self {
        let type_name = &resolved_type.type_name;
        if resolved_type.generic_params.is_empty() {
            return quote! { #type_name };
        }

        let generic_params = resolved_type.generic_params.iter().map(TokenStream::from);

        quote! { #type_name<#( #generic_params ),*> }
    }
}

impl From<ResolvedType> for TokenStream {
    fn from(resolved_type: ResolvedType) -> Self {
        (&resolved_type).into()
    }
}

/// Given a type, will recursively proceed to resolve it until it results in a
/// `ResolvedType` which can be then be converted into a `TokenStream`. As such
/// it can be used whenever you need the Rust type of the given
/// `TypeApplication`.
pub(crate) fn resolve_type(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let recursively_resolve = |type_applications: &Option<Vec<TypeApplication>>| {
        type_applications
            .iter()
            .flatten()
            .map(|array_type| resolve_type(array_type, types))
            .collect::<Result<Vec<_>, _>>()
    };

    let base_type = types.get(&type_application.type_id).unwrap();

    let components = recursively_resolve(&base_type.components)?;
    let type_arguments = recursively_resolve(&type_application.type_arguments)?;
    let type_field = base_type.type_field.as_str();

    [
        to_simple_type,
        to_byte,
        to_bits256,
        to_generic,
        to_array,
        to_sized_ascii_string,
        to_tuple,
        to_struct,
    ]
    .into_iter()
    .filter_map(|fun| fun(type_field, &components, &type_arguments))
    .next()
    .ok_or_else(|| Error::InvalidType(format!("Could not resolve {type_field} to any known type")))
}

fn to_generic(field: &str, _: &[ResolvedType], _: &[ResolvedType]) -> Option<ResolvedType> {
    let name = extract_generic_name(field)?;

    let type_name = safe_ident(&name).into_token_stream();
    Some(ResolvedType {
        type_name,
        generic_params: vec![],
    })
}

fn to_array(field: &str, components: &[ResolvedType], _: &[ResolvedType]) -> Option<ResolvedType> {
    let len = extract_array_len(field)?;

    let type_inside: TokenStream = match components {
        [single_type] => Ok(single_type.into()),
        _ => Err(Error::InvalidData(format!(
            "Array must have only one component! Actual components: {components:?}"
        ))),
    }
    .unwrap();

    Some(ResolvedType {
        type_name: quote! { [#type_inside; #len] },
        generic_params: vec![],
    })
}

fn to_sized_ascii_string(
    field: &str,
    _: &[ResolvedType],
    _: &[ResolvedType],
) -> Option<ResolvedType> {
    let len = extract_str_len(field)?;

    let generic_params = vec![ResolvedType {
        type_name: quote! {#len},
        generic_params: vec![],
    }];

    Some(ResolvedType {
        type_name: quote! { SizedAsciiString },
        generic_params,
    })
}

fn to_tuple(field: &str, components: &[ResolvedType], _: &[ResolvedType]) -> Option<ResolvedType> {
    if has_tuple_format(field) {
        let inner_types = components.iter().map(TokenStream::from);

        // it is important to leave a trailing comma because a tuple with
        // one element is written as (element,) not (element) which is
        // resolved to just element
        Some(ResolvedType {
            type_name: quote! {(#(#inner_types,)*)},
            generic_params: vec![],
        })
    } else {
        None
    }
}

fn to_simple_type(
    type_field: &str,
    _: &[ResolvedType],
    _: &[ResolvedType],
) -> Option<ResolvedType> {
    match type_field {
        "u8" | "u16" | "u32" | "u64" | "bool" | "()" => {
            let type_name = type_field
                .parse()
                .expect("Couldn't resolve primitive type. Cannot happen!");

            Some(ResolvedType {
                type_name,
                generic_params: vec![],
            })
        }
        _ => None,
    }
}

fn to_byte(type_field: &str, _: &[ResolvedType], _: &[ResolvedType]) -> Option<ResolvedType> {
    if type_field == "byte" {
        let type_name = quote! {Byte};
        Some(ResolvedType {
            type_name,
            generic_params: vec![],
        })
    } else {
        None
    }
}
fn to_bits256(type_field: &str, _: &[ResolvedType], _: &[ResolvedType]) -> Option<ResolvedType> {
    if type_field == "b256" {
        let type_name = quote! {Bits256};
        Some(ResolvedType {
            type_name,
            generic_params: vec![],
        })
    } else {
        None
    }
}

fn to_struct(
    field_name: &str,
    _: &[ResolvedType],
    type_arguments: &[ResolvedType],
) -> Option<ResolvedType> {
    custom_type_name(field_name)
        .ok()
        .map(|type_name| ident(&type_name))
        .map(|type_name| ResolvedType {
            type_name: type_name.into_token_stream(),
            generic_params: type_arguments.to_vec(),
        })
}
