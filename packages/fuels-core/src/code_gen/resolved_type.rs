use crate::code_gen::custom_types::extract_custom_type_name_from_abi_property;

use fuels_types::errors::Error;

use fuels_types::{TypeApplication, TypeDeclaration};
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::quote;
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

fn try_to_get_generic_name(field: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*generic\s+(\S+)\s*$").unwrap();
    }
    RE.captures(field)
        .map(|captures| String::from(&captures[1]))
}

fn try_to_get_array_length(field: &str) -> Option<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*\[.+;\s*(\d+)\s*\]\s*$").unwrap();
    }
    RE.captures(field)
        .map(|captures| captures[1].to_string())
        .map(|length| {
            str::parse(&length)
                .unwrap_or_else(|_| panic!("Could not extract array length from {length}!"))
        })
}

fn try_to_get_string_length(field: &str) -> Option<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^str\[(\d+)\]$").unwrap();
    }
    RE.captures(field)
        .map(|captures| captures[1].to_string())
        .map(|length| {
            str::parse(&length)
                .unwrap_or_else(|_| panic!("Could not extract array length from {length}!"))
        })
}

fn is_tuple(field: &str) -> bool {
    field.starts_with('(') && field.ends_with(')')
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

    something(base_type, components, type_arguments)
}

fn is_primitive(type_field: &str) -> TokenStream {}

fn something(
    base_type: &TypeDeclaration,
    components: Vec<ResolvedType>,
    type_arguments: Vec<ResolvedType>,
) -> anyhow::Result<ResolvedType, Error> {
    let type_field = base_type.type_field.as_str();
    let (type_name, generic_params) = match type_field {
        "u8" | "u16" | "u32" | "u64" | "bool" | "()" => Ok((
            type_field
                .parse()
                .expect("Couldn't resolve primitive type. Cannot happen!"),
            vec![],
        )),
        "byte" => Ok((quote! {Byte}, vec![])),
        "b256" => Ok((quote! {Bits256}, vec![])),
        _ => {
            if let Some(name) = try_to_get_generic_name(type_field) {
                let token_stream = name.parse().expect("Failed to parse generic param name");
                Ok::<_, Error>((token_stream, vec![]))
            } else if let Some(len) = try_to_get_array_length(type_field) {
                let type_inside: TokenStream = match components.as_slice() {
                    [single_type] => Ok(single_type.into()),
                    _ => Err(Error::InvalidData(format!(
                        "Array must have only one component! Actual components: {components:?}"
                    ))),
                }?;

                Ok((quote! { [#type_inside; #len] }, vec![]))
            } else if let Some(len) = try_to_get_string_length(type_field) {
                let generic_params = vec![ResolvedType {
                    type_name: quote! {#len},
                    generic_params: vec![],
                }];
                Ok((quote! { SizedAsciiString }, generic_params))
            } else if is_tuple(type_field) {
                let inner_types = components.into_iter().map(TokenStream::from);

                // it is important to leave a trailing comma because a tuple with
                // one element is written as (element,) not (element) which is
                // resolved to just element
                Ok((quote! {(#(#inner_types,)*)}, vec![]))
            } else if let Ok(type_name) = extract_custom_type_name_from_abi_property(base_type) {
                Ok((quote! {#type_name}, type_arguments))
            } else {
                panic!("resolve_type: Could not resolve {type_field}")
            }
        }
    }?;

    Ok(ResolvedType {
        type_name,
        generic_params,
    })
}
