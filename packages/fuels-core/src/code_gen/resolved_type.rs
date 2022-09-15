use crate::code_gen::custom_types::extract_custom_type_name_from_abi_property;
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use fuels_types::{TypeApplication, TypeDeclaration};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

// Represents a type alongside its generic parameters. Can be converted into a
// `TokenStream` via `.into()`.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedType {
    pub type_name: TokenStream,
    pub generic_params: Vec<ResolvedType>,
    pub param_type: ParamType,
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
    let param_type = ParamType::from_type_declaration(base_type, types)?;

    let (type_name, generic_params) = match &param_type {
        ParamType::Generic(name) => {
            let token_stream = name.parse().expect("Failed to parse generic param name");
            Ok::<_, Error>((token_stream, vec![]))
        }
        ParamType::U8 => Ok((quote! {u8}, vec![])),
        ParamType::U16 => Ok((quote! {u16}, vec![])),
        ParamType::U32 => Ok((quote! {u32}, vec![])),
        ParamType::U64 => Ok((quote! {u64}, vec![])),
        ParamType::Bool => Ok((quote! {bool}, vec![])),
        ParamType::Byte => Ok((quote! {u8}, vec![])),
        ParamType::B256 => Ok((quote! {Bits256}, vec![])),
        ParamType::Unit => Ok((quote! {()}, vec![])),
        ParamType::Array(_, len) => {
            let array_components = recursively_resolve(&base_type.components)?;

            let type_inside: TokenStream = match array_components.as_slice() {
                [single_type] => single_type.into(),
                _ => {
                    return Err(Error::InvalidData(format!("Array must have only one component! Actual components: {array_components:?}")));
                }
            };

            Ok((quote! { [#type_inside; #len] }, vec![]))
        }
        ParamType::String(len) => Ok((
            quote! { SizedAsciiString },
            vec![ResolvedType {
                type_name: quote! {#len},
                generic_params: vec![],
                param_type: ParamType::U64,
            }],
        )),
        ParamType::Struct(_) | ParamType::Enum(_) => {
            let type_name = extract_custom_type_name_from_abi_property(base_type)?;
            let generic_params = recursively_resolve(&type_application.type_arguments)?;
            Ok((quote! {#type_name}, generic_params))
        }
        ParamType::Tuple(_) => {
            let inner_types = recursively_resolve(&base_type.components)?
                .into_iter()
                .map(TokenStream::from);

            // it is important to leave a trailing comma because a tuple with
            // one element is written as (element,) not (element) which is
            // resolved to just element
            Ok((quote! {(#(#inner_types,)*)}, vec![]))
        }
    }?;

    Ok(ResolvedType {
        type_name,
        generic_params,
        param_type,
    })
}
