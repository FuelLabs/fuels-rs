use std::collections::HashMap;
use std::fmt::Debug;
use std::vec::IntoIter;

use itertools::{chain, Itertools};
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input::ParseMacroInput;
use syn::{
    punctuated::Punctuated, spanned::Spanned, AttributeArgs, Error, Lit, Lit::Str, LitStr,
    Meta::List, Meta::NameValue, MetaList, MetaNameValue, NestedMeta, NestedMeta::Meta,
};

use fuels_core::utils::ident;

pub(crate) fn parse_commands(input: ParseStream) -> syn::Result<Vec<Command>> {
    AttributeArgs::parse(input)?
        .into_iter()
        .map(Command::new)
        .collect()
}

#[derive(Debug)]
pub(crate) struct Command {
    pub(crate) name: Ident,
    pub(crate) contents: Punctuated<NestedMeta, syn::token::Comma>,
}

impl Command {
    pub fn new(nested_meta: NestedMeta) -> syn::Result<Self> {
        if let Meta(List(MetaList { path, nested, .. })) = nested_meta {
            let name = path.get_ident().cloned().ok_or_else(|| {
                Error::new_spanned(path, "Command name cannot be a Path -- i.e. contain ':'.")
            })?;
            Ok(Self {
                name,
                contents: nested,
            })
        } else {
            Err(Error::new_spanned(
                nested_meta,
                "Expected a command name literal -- e.g. `Something(...)`",
            ))
        }
    }
}

#[derive(Debug)]
pub(crate) struct UniqueLitStrs {
    span: Span,
    lit_strs: Vec<LitStr>,
}

impl UniqueLitStrs {
    pub(crate) fn new<T: ToTokens>(nested_metas: Punctuated<NestedMeta, T>) -> Result<Self, Error> {
        let span = nested_metas.span();

        let (lit_strs, errors): (Vec<_>, Vec<_>) = nested_metas
            .into_iter()
            .map(|meta| {
                if let NestedMeta::Lit(Str(lit_str)) = meta {
                    Ok(lit_str)
                } else {
                    Err(Error::new_spanned(meta, "Expected a string!"))
                }
            })
            .partition_result();

        if let Some(error) = combine_errors(errors) {
            return Err(error);
        }

        validate_no_duplicates(&lit_strs, |e| e.value().clone())?;

        Ok(Self { span, lit_strs })
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &LitStr> {
        self.lit_strs.iter()
    }

    pub(crate) fn span(&self) -> Span {
        self.span
    }
}

impl IntoIterator for UniqueLitStrs {
    type Item = LitStr;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.lit_strs.into_iter()
    }
}

pub(crate) struct UniqueNameValues {
    span: Span,
    name_values: HashMap<Ident, Lit>,
}

impl UniqueNameValues {
    pub(crate) fn new<T: ToTokens>(nested_metas: Punctuated<NestedMeta, T>) -> syn::Result<Self> {
        let span = nested_metas.span();
        let name_values = Self::extract_name_values(nested_metas.into_iter())?;

        validate_no_duplicates(&name_values, |(name, _)| name.clone())?;

        Ok(Self {
            span,
            name_values: name_values.into_iter().collect(),
        })
    }

    fn extract_name_values<T: Iterator<Item = NestedMeta>>(
        nested_metas: T,
    ) -> syn::Result<Vec<(Ident, Lit)>> {
        let (name_values, name_value_errs): (Vec<_>, Vec<_>) = nested_metas
            .map(Self::extract_name_value)
            .partition_result();

        let (ident_values, name_format_errors): (Vec<_>, Vec<Error>) = name_values
            .into_iter()
            .map(|nv| {
                let ident = nv.path.get_ident().cloned().ok_or_else(|| {
                    Error::new_spanned(
                        nv.path,
                        "Attribute name cannot be a `Path` -- i.e. must not contain ':'.",
                    )
                })?;

                Ok((ident, nv.lit))
            })
            .partition_result();

        let maybe_error = combine_errors(chain!(name_value_errs, name_format_errors));
        if let Some(error) = maybe_error {
            Err(error)
        } else {
            Ok(ident_values)
        }
    }

    fn extract_name_value(meta: NestedMeta) -> syn::Result<MetaNameValue> {
        if let Meta(NameValue(nv)) = meta {
            Ok(nv)
        } else {
            Err(Error::new_spanned(meta, "Expected name='value'."))
        }
    }

    pub(crate) fn try_get(&self, name: &str) -> Option<&Lit> {
        self.name_values.get(&ident(name))
    }

    pub(crate) fn validate_has_no_other_names(&self, allowed_names: &[&str]) -> syn::Result<()> {
        let expected_names = allowed_names
            .iter()
            .map(|name| format!("'{name}'"))
            .join(", ");

        let maybe_error: Option<Error> = self
            .name_values
            .keys()
            .filter(|name| !allowed_names.contains(&name.to_string().as_str()))
            .map(|name| {
                Error::new_spanned(
                    name.clone(),
                    format!("Attribute '{name}' not recognized! Expected: {expected_names}."),
                )
            })
            .reduce(|mut errors, error| {
                errors.combine(error);
                errors
            });

        if let Some(error) = maybe_error {
            Err(error)
        } else {
            Ok(())
        }
    }

    pub(crate) fn get_as_lit_str(&self, name: &str) -> syn::Result<&LitStr> {
        let value = self
            .try_get(name)
            .ok_or_else(|| Error::new(self.span.clone(), format!("Missing argument '{name}'.")))?;

        if let Str(lit_str) = value {
            Ok(lit_str)
        } else {
            Err(Error::new_spanned(
                value.clone(),
                format!("Expected the attribute '{name}' to have a string value!"),
            ))
        }
    }
}

pub(crate) fn combine_errors<T: IntoIterator<Item = Error>>(errs: T) -> Option<Error> {
    errs.into_iter().reduce(|mut errors, error| {
        errors.combine(error);
        errors
    })
}

//
// #[derive(Debug)]
// struct Attribute {
//     name: Ident,
//     value: Lit,
// }
//
// impl Parse for Attribute {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         let meta_name_value = input.parse::<MetaNameValue>()?;
//
//         let name = meta_name_value.path.get_ident().cloned().ok_or_else(|| {
//             Error::new_spanned(meta_name_value.path, "Path attribute names are not supported! Use an Ident instead -- e.g `name=\"value\"`")
//         })?;
//
//         let value = meta_name_value.lit;
//
//         Ok(Self { name, value })
//     }
// }
//
// #[derive(Debug)]
// pub(crate) struct Command {
//     pub(crate) name: Ident,
//     pub(crate) contents: Vec<NestedMeta>,
// }
//
// #[derive(Debug)]
// pub(crate) struct Attributes {
//     span: Span,
//     name_values: HashMap<String, Attribute>,
// }
//
// #[derive(Debug)]
// pub(crate) struct UniqueStringValues {
//     span: Span,
//     values: Vec<LitStr>,
// }
//
// impl Parse for UniqueStringValues {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         let span = input.span();
//
//         let values = Punctuated::<LitStr, Token![,]>::parse_terminated(&input)?
//             .into_iter()
//             .collect::<Vec<_>>();
//
//         validate_no_duplicates(&values, |t| t.value())?;
//
//         Ok(Self { span, values })
//     }
// }
//
// impl Attributes {
//     pub(crate) fn new() -> {
//         let span = input.span();
//         let name_values = input
//             .parse_terminated::<_, Token![,]>(Attribute::parse)?
//             .into_iter()
//             .collect::<Vec<_>>();
//
//         validate_no_duplicates(&name_values, |t| t.name.clone())?;
//
//         let name_values = name_values
//             .into_iter()
//             .map(|nv| (nv.name.to_string(), nv))
//             .collect();
//
//         Ok(Self { span, name_values })
//     }
//     pub(crate) fn try_get(&self, name: &str) -> Option<&Lit> {
//         self.name_values.get(name).map(|nv| &nv.value)
//     }
//
//     pub(crate) fn validate_has_no_other_names(&self, allowed_names: &[&str]) -> syn::Result<()> {
//         let expected_names = allowed_names
//             .iter()
//             .map(|name| format!("'{name}'"))
//             .join(", ");
//
//         let maybe_error: Option<Error> = self
//             .name_values
//             .keys()
//             .filter(|name| !allowed_names.contains(&name.to_string().as_str()))
//             .map(|name| {
//                 Error::new_spanned(
//                     name.clone(),
//                     format!("Attribute '{name}' not recognized! Expected: {expected_names}."),
//                 )
//             })
//             .reduce(|mut errors, error| {
//                 errors.combine(error);
//                 errors
//             });
//
//         if let Some(error) = maybe_error {
//             Err(error)
//         } else {
//             Ok(())
//         }
//     }
//
//     pub(crate) fn get_as_lit_str(&self, name: &str) -> syn::Result<&LitStr> {
//         let value = self
//             .try_get(name)
//             .ok_or_else(|| Error::new(self.span.clone(), format!("Missing argument '{name}'.")))?;
//
//         if let Lit::Str(lit_str) = value {
//             Ok(lit_str)
//         } else {
//             Err(Error::new_spanned(
//                 value.clone(),
//                 format!("Expected the attribute '{name}' to have a string value!"),
//             ))
//         }
//     }
// }

fn generate_duplicate_error<T, K, KeyFn>(duplicates: &[&T], key_fn: KeyFn) -> Error
where
    KeyFn: Fn(&&T) -> K,
    K: ToTokens,
{
    duplicates
        .iter()
        .map(|duplicate| Error::new_spanned(key_fn(duplicate), "Duplicate!"))
        .reduce(|mut errors, error| {
            errors.combine(error);
            errors
        })
        .expect("to have at least one duplicate here")
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
    K: Ord + ToTokens,
{
    let maybe_err = group_up_duplicates(elements, key_fn)
        .into_iter()
        .map(|duplicates| generate_duplicate_error(&duplicates, key_fn))
        .reduce(|mut errors, error| {
            errors.combine(error);
            errors
        });

    if let Some(err) = maybe_err {
        Err(err)
    } else {
        Ok(())
    }
}

// impl Parse for Attributes {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//
//     }
// }
//
// impl<T: Parse + Debug> Parse for Command<T> {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         let name = input.parse::<Ident>()?;
//
//         let content;
//         parenthesized!(content in input);
//
//         let contents = T::parse(&content)?;
//
//         Ok(Self { name, contents })
//     }
// }

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse::ParseStream;
    use syn::parse_macro_input::ParseMacroInput;
    use syn::AttributeArgs;

    use crate::experimental::{Command, UniqueLitStrs};

    // use quote::quote;
    // use syn::{
    //     parse::{Parse, ParseStream},
    //     Error, Lit, LitStr, MetaNameValue, Token,
    // };
    //
    // use super::*;
    //
    // #[test]
    // fn something() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let command: Command<LitStr> = Parse::parse(input)?;
    //
    //             dbg!(command);
    //             Ok(())
    //         },
    //         quote! {ACommand("some_value", "here")},
    //     )?;
    //     Ok(())
    // }
    //
    // #[test]
    // fn something2() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let command: Command<NameValue> = Parse::parse(input)?;
    //
    //             dbg!(command);
    //             Ok(())
    //         },
    //         quote! {ACommand(name="some_value", other="here")},
    //     )?;
    //
    //     Ok(())
    // }
    // use crate::attributes::Attributes;
    // use quote::quote;
    // use syn::parse::ParseStream;
    //
    // #[test]
    // fn fails_if_attr_names_are_not_recognized() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let err =
    //                 Attributes::new(input, &["name", "abi"]).expect_err("Should have failed.");
    //
    //             assert_eq!(
    //                 err.to_string(),
    //                 "Unknown attribute! Expected one of: 'name', 'abi'."
    //             );
    //
    //             Ok(())
    //         },
    //         quote! {(name = "some_value", some_typo="here")},
    //     )?;
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn fails_if_name_or_abi_values_are_not_strings() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let attributes = Attributes::new(input, &["name", "abi"])?;
    //
    //             {
    //                 let err = attributes
    //                     .get_as_str("name")
    //                     .expect_err("Should have failed.");
    //                 assert_eq!(
    //                     err.to_string(),
    //                     "Expected a string for the value of the 'name' attribute."
    //                 )
    //             }
    //             {
    //                 let err = attributes
    //                     .get_as_str("abi")
    //                     .expect_err("Should have failed.");
    //                 assert_eq!(
    //                     err.to_string(),
    //                     "Expected a string for the value of the 'abi' attribute."
    //                 )
    //             }
    //
    //             Ok(())
    //         },
    //         quote! {(name = 123, abi=true)},
    //     )?;
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn fails_if_names_are_duplicated() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let err =
    //                 Attributes::new(input, &["name", "abi"]).expect_err("Should have failed.");
    //
    //             assert_eq!(
    //                 err.to_string(),
    //                 "Duplicate attribute 'name'! Original defined here:"
    //             );
    //
    //             Ok(())
    //         },
    //         quote! {(name = "something", abi="else", name="something")},
    //     )?;
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn can_extract_attr_values() -> syn::Result<()> {
    //     syn::parse::Parser::parse2(
    //         |input: ParseStream<'_>| {
    //             let sut = Attributes::new(input, &["name", "abi"])?;
    //
    //             {
    //                 let value = sut.get_as_str("name").unwrap();
    //
    //                 assert_eq!(value, "some_value");
    //             }
    //             {
    //                 let value = sut.get_as_str("abi").unwrap();
    //
    //                 assert_eq!(value, "some_abi");
    //             }
    //
    //             Ok(())
    //         },
    //         quote! {(name = "some_value", abi = "some_abi")},
    //     )
    // }

    #[test]
    fn can_extract_attr_values() -> syn::Result<()> {
        syn::parse::Parser::parse2(
            |input: ParseStream<'_>| {
                let span = input.span();
                let result = AttributeArgs::parse(input)?;
                let command = Command::new(result.last().unwrap().clone())?;
                let punctuated = command.contents;
                let values = UniqueLitStrs::new(punctuated)?;
                dbg!(values);

                Ok(())
            },
            quote! {Something(name = "some_value", rg="some_abi"), Else("abc", "efg")},
        )?;
        panic!("Just to get the messages");
    }
}
