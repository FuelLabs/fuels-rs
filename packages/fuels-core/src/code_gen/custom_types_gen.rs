use crate::code_gen::functions_gen::{resolve_type, ResolvedType};
use crate::utils::ident;
use crate::{try_from_bytes, ParamType, Parameterize, Token, Tokenizable};
use anyhow::{anyhow, bail};
use fuels_types::errors::Error;
use fuels_types::param_types::EnumVariants;
use fuels_types::utils::has_array_format;
use fuels_types::{CustomType, TypeApplication, TypeDeclaration};
use inflector::Inflector;
use itertools::Itertools;
use lazy_static::lazy_static;
use proc_macro2::{Ident, LexError, TokenStream};
use quote::{quote, ToTokens};
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use std::iter::Map;
use std::slice::Iter;
use std::str::FromStr;
use syn::parse_macro_input;

struct Component {
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

pub fn expand_custom_struct(
    prop: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let struct_ident = extract_custom_type_name_from_abi_property(prop)?;

    let field_entries = extract_components(&prop, types, true)?;
    let generic_parameters = extract_generic_parameters(&field_entries)?;

    let struct_decl = struct_decl(&struct_ident, &field_entries, &generic_parameters);

    let parameterized_impl =
        struct_parameterized_impl(&field_entries, &struct_ident, &generic_parameters);

    let tokenizable_impl =
        struct_tokenizable_impl(&struct_ident, &field_entries, &generic_parameters);

    let try_from = impl_try_from(&struct_ident, &generic_parameters);

    Ok(quote! {
        #struct_decl

        #parameterized_impl

        #tokenizable_impl

        #try_from
    })
}

pub fn expand_custom_enum(
    prop: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let enum_ident = &extract_custom_type_name_from_abi_property(prop)?;

    let field_entries = extract_components(&prop, types, false)?;
    let generics = extract_generic_parameters(&field_entries)?;

    let enum_def = enum_decl(&enum_ident, &field_entries, &generics);
    let parameterize_impl = enum_parameterize_impl(&enum_ident, &field_entries, &generics);
    let tokenize_impl = enum_tokenizable_impl(&enum_ident, &field_entries, &generics);
    let try_from = impl_try_from(&enum_ident, &generics);

    Ok(quote! {
        #enum_def

        #parameterize_impl

        #tokenize_impl

        #try_from
    })
}

fn impl_try_from(ident: &Ident, generics: &[TokenStream]) -> TokenStream {
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

fn enum_decl(
    enum_ident: &Ident,
    field_entries: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let enum_variants = field_entries.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            let field_type = if let ParamType::Unit = field_type.param_type {
                quote! {}
            } else {
                field_type.into()
            };

            quote! {
                #field_name(#field_type)
            }
        },
    );

    quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum #enum_ident <#(#generics: Tokenizable + Parameterize,)*> {
            #(#enum_variants,)*
        }
    }
}

fn enum_tokenizable_impl(
    enum_ident: &Ident,
    field_entries: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let enum_name = enum_ident.to_string();

    let match_discriminant_from_token = field_entries.iter().enumerate().map(
        |(
            discriminant,
            Component {
                field_name,
                field_type,
            },
        )| {
            let value = if let ParamType::Unit = field_type.param_type {
                quote! {}
            } else {
                let field_type: TokenStream = field_type.into();
                quote! { <#field_type>::from_token(variant_token)? }
            };

            let u8_discriminant = discriminant as u8;
            quote! { #u8_discriminant => Ok(Self::#field_name(#value))}
        },
    );

    let match_discriminant_into_token = field_entries.iter().enumerate().map(
        |(
            discriminant,
            Component {
                field_name,
                field_type,
            },
        )| {
            let u8_discriminant = discriminant as u8;
            if let ParamType::Unit = field_type.param_type {
                quote! { Self::#field_name() => (#u8_discriminant, ().into_token())}
            } else {
                quote! { Self::#field_name(inner) => (#u8_discriminant, inner.into_token())}
            }
        },
    );

    quote! {
            impl<#(#generics: Tokenizable + Parameterize,)*> Tokenizable for #enum_ident <#(#generics,)*> {
                fn from_token(token: Token) -> Result<Self, SDKError>
                where
                    Self: Sized,
                {
                    let gen_err = |msg| {
                        SDKError::InvalidData(format!(
                            "Error while instantiating {} from token! {}", #enum_name, msg
                        ))
                    };
                    match token {
                        Token::Enum(selector) => {
                            let (discriminant, variant_token, _) = *selector;
                            match discriminant {
                                #(#match_discriminant_from_token,)*
                                _ => Err(gen_err(format!(
                                    "Discriminant {} doesn't point to any of the enums variants.", discriminant
                                ))),
                            }
                        }
                        _ => Err(gen_err(format!(
                            "Given token ({}) is not of the type Token::Enum!", token
                        ))),
                    }
                }

                fn into_token(self) -> Token {
                    let (discriminant, token) = match self {
                        #(#match_discriminant_into_token,)*
                    };

                    let variants = match Self::param_type() {
                        ParamType::Enum(variants) => variants,
                        other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {}", #enum_name, other)
                    };

                    Token::Enum(Box::new((discriminant, token, variants)))
                }
            }
    }
}

fn enum_parameterize_impl(
    enum_ident: &Ident,
    field_entries: &[Component],
    generics: &[TokenStream],
) -> TokenStream {
    let param_type_calls = param_type_calls(&field_entries);
    let enum_name = enum_ident.to_string();
    quote! {
        impl<#(#generics: Parameterize + Tokenizable,)*> Parameterize for #enum_ident <#(#generics,)*> {
            fn param_type() -> ParamType {
                let mut param_types = vec![];
                #(param_types.push(#param_type_calls);)*

                let variants = EnumVariants::new(param_types).unwrap_or_else(|_| panic!("{} has no variants which isn't allowed!", #enum_name));
                ParamType::Enum(variants)
            }
        }
    }
}

fn struct_decl(
    struct_ident: &Ident,
    field_entries: &Vec<Component>,
    generic_parameters: &Vec<TokenStream>,
) -> TokenStream {
    let fields = field_entries.iter().map(
        |(Component {
             field_name,
             field_type,
         })| {
            let field_type: TokenStream = field_type.into();
            quote! { pub #field_name: #field_type }
        },
    );

    quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct #struct_ident <#(#generic_parameters: Tokenizable + Parameterize, )*> {
            #(#fields),*
        }
    }
}

fn struct_tokenizable_impl(
    struct_ident: &Ident,
    field_entries: &[Component],
    generic_parameters: &Vec<TokenStream>,
) -> TokenStream {
    let struct_name_str = struct_ident.to_string();
    let from_token_calls = field_entries
        .iter()
        .map(
            |(Component {
                 field_name,
                 field_type,
             })| {
                let resolved: TokenStream = field_type.into();
                quote! {
                    #field_name: <#resolved>::from_token(next_token()?)?
                }
            },
        )
        .collect::<Vec<_>>();

    let into_token_calls = field_entries
        .iter()
        .map(|Component { field_name, .. }| {
            quote! {self.#field_name.into_token()}
        })
        .collect::<Vec<_>>();

    quote! {
        impl <#(#generic_parameters: Tokenizable + Parameterize, )*> Tokenizable for #struct_ident <#(#generic_parameters, )*> {
            fn into_token(self) -> Token {
                let mut tokens = Vec::new();
                #( tokens.push(#into_token_calls); )*
                Token::Struct(tokens)
            }

            fn from_token(token: Token)  -> Result<Self, SDKError> {
                match token {
                    Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { SDKError::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name_str)) })
                        };
                        Ok(Self { #( #from_token_calls, )* })
                    },
                    other => Err(SDKError::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    }
}

fn struct_parameterized_impl(
    field_entries: &[Component],
    struct_ident: &Ident,
    generic_parameters: &[TokenStream],
) -> TokenStream {
    let param_type_calls = param_type_calls(&field_entries);
    quote! {
        impl <#(#generic_parameters: Parameterize + Tokenizable,)*> Parameterize for #struct_ident <#(#generic_parameters,)*> {
            fn param_type() -> ParamType {
                let mut types = Vec::new();
                #( types.push(#param_type_calls); )*
                ParamType::Struct(types)
            }
        }
    }
}

fn param_type_calls(field_entries: &[Component]) -> Vec<TokenStream> {
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

fn extract_components(
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

fn extract_generic_parameters(field_types: &[Component]) -> Result<Vec<TokenStream>, LexError> {
    field_types
        .iter()
        .map(|Component { field_type, .. }| field_type.get_generic_types())
        .flatten()
        .unique()
        .map(|arg| arg.parse())
        .collect()
}

// A custom type name should be passed to this function as `{struct,enum} $name`,
pub(crate) fn extract_custom_type_name_from_abi_property(
    prop: &TypeDeclaration,
) -> Result<Ident, Error> {
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

// Doing string -> TokenStream -> string isn't pretty but gives us the opportunity to
// have a better understanding of the generated code so we consider it ok.
// To generate the expected examples, output of the functions were taken
// with code @9ca376, and formatted in-IDE using rustfmt. It should be noted that
// rustfmt added an extra `,` after the last struct/enum field, which is not added
// by the `expand_custom_*` functions, and so was removed from the expected string.
// TODO(vnepveu): append extra `,` to last enum/struct field so it is aligned with rustfmt
#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::ProgramABI;
    use std::str::FromStr;

    // TODO: Move tests using the old abigen to the new one.
    // Currently, they will be skipped. Even though we're not fully testing these at
    // unit level, they're tested at integration level, in the main harness.rs file.

    // #[test]
    // fn test_extract_custom_type_name_from_abi_property_bad_data() {
    //     let p: Property = Default::default();
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidData(_))));

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("nowhitespacehere"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidData(_))));
    // }

    // #[test]
    // fn test_extract_struct_name_from_abi_property_wrong_type() {
    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("enum something"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
    //     assert!(matches!(result, Err(Error::InvalidType(_))));

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("struct somethingelse"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidType(_))));
    // }

    // #[test]
    // fn test_extract_custom_type_name_from_abi_property() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("struct bar"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
    //     assert_eq!(result?, "bar");

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("enum bar"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert_eq!(result?, "bar");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_enum() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("LongIsland"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("MoscowMule"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_enum("MatchaTea", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum MatchaTea { LongIsland (u64) , MoscowMule (bool) } impl Parameterize for MatchaTea { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: U64) ; types . push (ParamType :: Bool) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "MatchaTea" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for MatchaTea { fn into_token (self) -> Token { let (dis , tok) = match self { MatchaTea :: LongIsland (value) => (0u8 , Token :: U64 (value)) , MatchaTea :: MoscowMule (value) => (1u8 , Token :: Bool (value)) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => Ok (MatchaTea :: LongIsland (< u64 > :: from_token (token) ?)) , (1u8 , token , _) => Ok (MatchaTea :: MoscowMule (< bool > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "MatchaTea" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "MatchaTea" , token))) } } } impl TryFrom < & [u8] > for MatchaTea { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MatchaTea { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MatchaTea { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn top_lvl_enum_w_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
    //     assert_enum_cannot_be_constructed_from(Some(vec![]))?;
    //     assert_enum_cannot_be_constructed_from(None)?;
    //     Ok(())
    // }
    // #[test]
    // fn nested_enum_w_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
    //     let nested_enum_w_components = |components| {
    //         Some(vec![Property {
    //             name: "SomeEmptyEnum".to_string(),
    //             type_field: "enum SomeEmptyEnum".to_string(),
    //             components,
    //         }])
    //     };

    //     assert_enum_cannot_be_constructed_from(nested_enum_w_components(None))?;
    //     assert_enum_cannot_be_constructed_from(nested_enum_w_components(Some(vec![])))?;

    //     Ok(())
    // }

    // fn assert_enum_cannot_be_constructed_from(
    //     components: Option<Vec<Property>>,
    // ) -> anyhow::Result<()> {
    //     let property = Property {
    //         components,
    //         ..Property::default()
    //     };

    //     let err = expand_custom_enum("TheEmptyEnum", &property)
    //         .err()
    //         .ok_or_else(|| anyhow!("Was able to construct an enum without variants"))?;

    //     assert!(
    //         matches!(err, Error::InvalidType(_)),
    //         "Expected the error to be of the type 'InvalidType'"
    //     );

    //     Ok(())
    // }

    // #[test]
    // fn test_expand_struct_inside_enum() -> Result<(), Error> {
    //     let inner_struct = Property {
    //         name: String::from("Infrastructure"),
    //         type_field: String::from("struct Building"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("Rooms"),
    //                 type_field: String::from("u8"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("Floors"),
    //                 type_field: String::from("u16"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let enum_components = vec![
    //         inner_struct,
    //         Property {
    //             name: "Service".to_string(),
    //             type_field: "u32".to_string(),
    //             components: None,
    //         },
    //     ];
    //     let p = Property {
    //         name: String::from("CityComponent"),
    //         type_field: String::from("enum CityComponent"),
    //         components: Some(enum_components),
    //     };
    //     let actual = expand_custom_enum("Amsterdam", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum Amsterdam { Infrastructure (Building) , Service (u32) } impl Parameterize for Amsterdam { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (Building :: param_type ()) ; types . push (ParamType :: U32) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "Amsterdam" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for Amsterdam { fn into_token (self) -> Token { let (dis , tok) = match self { Amsterdam :: Infrastructure (inner_struct) => (0u8 , inner_struct . into_token ()) , Amsterdam :: Service (value) => (1u8 , Token :: U32 (value)) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < Building > :: from_token (token) ? ; Ok (Amsterdam :: Infrastructure (variant_content)) } (1u8 , token , _) => Ok (Amsterdam :: Service (< u32 > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "Amsterdam" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "Amsterdam" , token))) } } } impl TryFrom < & [u8] > for Amsterdam { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Amsterdam { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Amsterdam { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_array_inside_enum() -> Result<(), Error> {
    //     let enum_components = vec![Property {
    //         name: "SomeArr".to_string(),
    //         type_field: "[u64; 7]".to_string(),
    //         components: None,
    //     }];
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(enum_components),
    //     };
    //     let actual = expand_custom_enum("SomeEnum", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum SomeEnum { SomeArr (:: std :: vec :: Vec < u64 >) } impl Parameterize for SomeEnum { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Array (Box :: new (ParamType :: U64) , 7)) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "SomeEnum" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for SomeEnum { fn into_token (self) -> Token { let (dis , tok) = match self { SomeEnum :: SomeArr (value) => (0u8 , Token :: Array (vec ! [value . into_token ()])) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => Ok (SomeEnum :: SomeArr (< :: std :: vec :: Vec < u64 > > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "SomeEnum" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "SomeEnum" , token))) } } } impl TryFrom < & [u8] > for SomeEnum { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for SomeEnum { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for SomeEnum { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_enum_with_enum() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(vec![Property {
    //             name: String::from("El2"),
    //             type_field: String::from("enum EnumLevel2"),
    //             components: Some(vec![Property {
    //                 name: String::from("El1"),
    //                 type_field: String::from("enum EnumLevel1"),
    //                 components: Some(vec![Property {
    //                     name: String::from("Num"),
    //                     type_field: String::from("u32"),
    //                     components: None,
    //                 }]),
    //             }]),
    //         }]),
    //     };
    //     let actual = expand_custom_enum("EnumLevel3", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum EnumLevel3 { El2 (EnumLevel2) } impl Parameterize for EnumLevel3 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (EnumLevel2 :: param_type ()) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "EnumLevel3" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for EnumLevel3 { fn into_token (self) -> Token { let (dis , tok) = match self { EnumLevel3 :: El2 (inner_enum) => (0u8 , inner_enum . into_token ()) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < EnumLevel2 > :: from_token (token) ? ; Ok (EnumLevel3 :: El2 (variant_content)) } (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "EnumLevel3" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "EnumLevel3" , token))) } } } impl TryFrom < & [u8] > for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_struct() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Cocktail"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("long_island"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("cosmopolitan"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("mojito"),
    //                 type_field: String::from("u32"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_struct(&p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : bool , pub cosmopolitan : u64 , pub mojito : u32 } impl Parameterize for Cocktail { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Bool) ; types . push (ParamType :: U64) ; types . push (ParamType :: U32) ; ParamType :: Struct (types) } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: Bool (self . long_island)) ; tokens . push (Token :: U64 (self . cosmopolitan)) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : < bool > :: from_token (next_token () ?) ? , cosmopolitan : < u64 > :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl TryFrom < & [u8] > for Cocktail { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_struct_with_struct() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Cocktail"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("long_island"),
    //                 type_field: String::from("struct Shaker"),
    //                 components: Some(vec![
    //                     Property {
    //                         name: String::from("cosmopolitan"),
    //                         type_field: String::from("bool"),
    //                         components: None,
    //                     },
    //                     Property {
    //                         name: String::from("bimbap"),
    //                         type_field: String::from("u64"),
    //                         components: None,
    //                     },
    //                 ]),
    //             },
    //             Property {
    //                 name: String::from("mojito"),
    //                 type_field: String::from("u32"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_struct(&p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : Shaker , pub mojito : u32 } impl Parameterize for Cocktail { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (Shaker :: param_type ()) ; types . push (ParamType :: U32) ; ParamType :: Struct (types) } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (self . long_island . into_token ()) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : Shaker :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl TryFrom < & [u8] > for Cocktail { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // TODO: FIX ME
    //     #[test]
    //     fn test_expand_struct_new_abi() -> Result<(), Error> {
    //         let s = r#"
    //         {
    //             "types": [
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 10,
    //                 "type": "bool",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 12,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 2,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 3,
    //                 "type": "struct MyStruct2",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 10,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 12,
    //                     "typeArguments": []
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 26,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               }
    //             ],
    //             "functions": [
    //               {
    //                 "type": "function",
    //                 "inputs": [
    //                   {
    //                     "name": "s1",
    //                     "type": 2,
    //                     "typeArguments": []
    //                   },
    //                   {
    //                     "name": "s2",
    //                     "type": 3,
    //                     "typeArguments": []
    //                   }
    //                 ],
    //                 "name": "some_abi_funct",
    //                 "output": {
    //                   "name": "",
    //                   "type": 26,
    //                   "typeArguments": []
    //                 }
    //               }
    //             ]
    //           }
    // "#;
    //         let parsed_abi: ProgramABI = serde_json::from_str(s)?;
    //         let all_types = parsed_abi
    //             .types
    //             .into_iter()
    //             .map(|t| (t.type_id, t))
    //             .collect::<HashMap<usize, TypeDeclaration>>();
    //
    //         let s1 = all_types.get(&2).unwrap();
    //
    //         let actual = expand_custom_struct(s1, &all_types)?.to_string();
    //
    //         let expected = TokenStream::from_str(
    //             r#"
    //             # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct1 { pub x : u64 , pub y : [u8 ; 32] } impl Parameterize for MyStruct1 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: U64) ; types . push (ParamType :: B256) ; ParamType :: Struct (types) } } impl Tokenizable for MyStruct1 { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: U64 (self . x)) ; tokens . push (Token :: B256 (self . y)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct1")) }) } ; Ok (Self { x : < u64 > :: from_token (next_token () ?) ? , y : < [u8 ; 32] > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct1" , other))) , } } } impl TryFrom < & [u8] > for MyStruct1 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MyStruct1 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MyStruct1 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //             "#,
    //         )?.to_string();
    //
    //         assert_eq!(actual, expected);
    //
    //         let s2 = all_types.get(&3).unwrap();
    //
    //         let actual = expand_custom_struct(s2, &all_types)?.to_string();
    //
    //         let expected = TokenStream::from_str(
    //             r#"
    //             # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct2 { pub x : bool , pub y : MyStruct1 } impl Parameterize for MyStruct2 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Bool) ; types . push (MyStruct1 :: param_type ()) ; ParamType :: Struct (types) } } impl Tokenizable for MyStruct2 { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: Bool (self . x)) ; tokens . push (self . y . into_token ()) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct2")) }) } ; Ok (Self { x : < bool > :: from_token (next_token () ?) ? , y : MyStruct1 :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct2" , other))) , } } } impl TryFrom < & [u8] > for MyStruct2 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MyStruct2 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MyStruct2 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //             "#,
    //         )?.to_string();
    //
    //         assert_eq!(actual, expected);
    //
    //         Ok(())
    //     }
}
