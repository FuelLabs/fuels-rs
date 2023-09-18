pub(crate) use command::Command;
use itertools::{chain, Itertools};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{DataEnum, DataStruct, Error, Fields, GenericParam, Generics, TypeParam, Variant};
pub(crate) use unique_lit_strs::UniqueLitStrs;
pub(crate) use unique_name_values::UniqueNameValues;

mod command;
mod unique_lit_strs;
mod unique_name_values;

pub(crate) trait ErrorsExt: Iterator<Item = Error> + Sized {
    fn combine_errors(self) -> Option<Self::Item>;
    fn validate_no_errors(self) -> Result<(), Self::Item>;
}

impl<T> ErrorsExt for T
where
    T: Iterator<Item = Error> + Sized,
{
    fn combine_errors(self) -> Option<Self::Item> {
        self.reduce(|mut errors, error| {
            errors.combine(error);
            errors
        })
    }

    fn validate_no_errors(self) -> Result<(), Self::Item> {
        if let Some(err) = self.combine_errors() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

fn generate_duplicate_error<T>(duplicates: &[&T]) -> Error
where
    T: ToTokens,
{
    let mut iter = duplicates.iter();

    let original_error = iter
        .next()
        .map(|first_el| Error::new_spanned(first_el, "Original defined here:"));

    let the_rest = iter.map(|duplicate| Error::new_spanned(duplicate, "Duplicate!"));

    chain!(original_error, the_rest)
        .combine_errors()
        .expect("Has to be at least one error!")
}

fn group_up_duplicates<T, K, KeyFn>(name_values: &[T], key: KeyFn) -> Vec<Vec<&T>>
where
    KeyFn: Fn(&&T) -> K,
    K: Ord,
{
    name_values
        .iter()
        .sorted_by_key(&key)
        .group_by(&key)
        .into_iter()
        .filter_map(|(_, group)| {
            let group = group.collect::<Vec<_>>();

            (group.len() > 1).then_some(group)
        })
        .collect()
}

fn validate_no_duplicates<T, K, KeyFn>(elements: &[T], key_fn: KeyFn) -> syn::Result<()>
where
    KeyFn: Fn(&&T) -> K + Copy,
    T: ToTokens,
    K: Ord,
{
    group_up_duplicates(elements, key_fn)
        .into_iter()
        .map(|duplicates| generate_duplicate_error(&duplicates))
        .validate_no_errors()
}

pub fn validate_and_extract_generic_types(generics: &Generics) -> syn::Result<Vec<&TypeParam>> {
    generics
        .params
        .iter()
        .map(|generic_param| match generic_param {
            GenericParam::Type(generic_type) => Ok(generic_type),
            GenericParam::Lifetime(lifetime) => {
                Err(Error::new_spanned(lifetime, "Lifetimes not supported"))
            }
            GenericParam::Const(const_generic) => Err(Error::new_spanned(
                const_generic,
                "Const generics not supported",
            )),
        })
        .collect()
}

struct Member {
    name: Ident,
    ty: TokenStream,
    ignore: bool,
}

pub(crate) struct Members {
    members: Vec<Member>,
    fuels_core_path: TokenStream,
}

impl Members {
    pub(crate) fn names(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.members
            .iter()
            .filter(|member| !member.ignore)
            .map(|member| &member.name)
    }

    pub(crate) fn ignored_names(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.members
            .iter()
            .filter(|member| member.ignore)
            .map(|member| &member.name)
    }

    pub(crate) fn param_type_calls(&self) -> impl Iterator<Item = TokenStream> + '_ {
        let fuels_core_path = self.fuels_core_path.to_token_stream();
        self.members
            .iter()
            .filter(|member| !member.ignore)
            .map(move |member| {
                let ty = &member.ty;
                quote! { <#ty as #fuels_core_path::traits::Parameterize>::param_type() }
            })
    }
}

pub(crate) fn extract_struct_members(
    fields: DataStruct,
    fuels_core_path: TokenStream,
) -> syn::Result<Members> {
    let named_fields = match fields.fields {
        Fields::Named(named_fields) => Ok(named_fields.named),
        Fields::Unnamed(fields) => Err(Error::new_spanned(
            fields.unnamed,
            "Tuple-like structs not supported",
        )),
        _ => {
            panic!("This cannot happen in valid Rust code. Fields::Unit only appears in enums")
        }
    }?;

    let members = named_fields
        .into_iter()
        .map(|field| {
            let ignore = field.attrs.iter().any(|attr| match &attr.meta {
                syn::Meta::Path(path) => path.get_ident().is_some_and(|ident| ident == "Ignore"),
                _ => false,
            });
            let name = field
                .ident
                .expect("FieldsNamed to only contain named fields.");
            let ty = field.ty.into_token_stream();
            Member { name, ty, ignore }
        })
        .collect();

    Ok(Members {
        members,
        fuels_core_path,
    })
}

pub(crate) fn extract_enum_members(
    data: DataEnum,
    fuels_core_path: TokenStream,
) -> syn::Result<Members> {
    let members = data
        .variants
        .into_iter()
        .filter(|variant| {
            !variant.attrs.iter().any(|attr| match &attr.meta {
                syn::Meta::Path(path) => path.get_ident().is_some_and(|ident| ident == "Ignore"),
                _ => false,
            })
        })
        .map(|variant: Variant| {
            let name = variant.ident;

            let ty = match variant.fields {
                Fields::Unnamed(fields_unnamed) => {
                    if fields_unnamed.unnamed.len() != 1 {
                        return Err(Error::new(
                            fields_unnamed.paren_token.span.join(),
                            "Must have exactly one element",
                        ));
                    }
                    fields_unnamed.unnamed.into_iter().next()
                }
                Fields::Unit => None,
                Fields::Named(named_fields) => {
                    return Err(Error::new_spanned(
                        named_fields,
                        "Struct-like enum variants are not supported.",
                    ))
                }
            }
            .map(|field| field.ty.into_token_stream())
            .unwrap_or_else(|| quote! {()});

            Ok(Member {
                name,
                ty,
                ignore: false,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Members {
        members,
        fuels_core_path,
    })
}
