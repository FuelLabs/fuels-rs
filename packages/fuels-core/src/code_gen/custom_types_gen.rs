use crate::errors::Error;
use crate::json_abi::parse_param;
use crate::types::expand_type;
use crate::utils::ident;
use crate::ParamType;
use fuels_types::{CustomType, Property};
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;

/// Functions used by the Abigen to expand custom types defined in an ABI spec.

/// Transforms a custom type defined in [`Property`] into a [`TokenStream`]
/// that represents that same type as a Rust-native struct.
pub fn expand_custom_struct(prop: &Property) -> Result<TokenStream, Error> {
    let struct_name = &extract_custom_type_name_from_abi_property(prop, Some(CustomType::Struct))?
        .to_class_case();
    let struct_ident = ident(struct_name);
    let components = prop.components.as_ref().unwrap();
    let mut fields = Vec::with_capacity(components.len());

    // Holds a TokenStream representing the process of
    // creating a [`Token`] and pushing it a vector of Tokens.
    let mut struct_fields_tokens = Vec::new();
    let mut param_types = Vec::new();

    // Holds the TokenStream representing the process
    // of creating a Self struct from each `Token`.
    // Used when creating a struct from tokens with
    // `MyStruct::new_from_tokens()`.
    let mut args = Vec::new();

    // For each component, we create two TokenStreams:
    // 1. A struct field declaration like `pub #field_name: #component_name`
    // 2. The creation of a token and its insertion into a vector of Tokens.
    for (idx, component) in components.iter().enumerate() {
        let field_name = ident(&component.name.to_snake_case());
        let param_type = parse_param(component)?;

        match param_type {
            // Case where a struct takes another struct
            ParamType::Struct(_params) => {
                let inner_struct_ident = ident(
                    &extract_custom_type_name_from_abi_property(
                        component,
                        Some(CustomType::Struct),
                    )?
                    .to_class_case(),
                );

                fields.push(quote! {pub #field_name: #inner_struct_ident});
                args.push(
                    quote! {#field_name: #inner_struct_ident::new_from_tokens(&tokens[#idx..])},
                );
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
                param_types.push(
                    quote! { types.push(ParamType::Struct(#inner_struct_ident::param_types())) },
                );
            }
            // The struct contains a nested enum
            ParamType::Enum(_params) => {
                let enum_name = ident(
                    &extract_custom_type_name_from_abi_property(component, Some(CustomType::Enum))?
                        .to_class_case(),
                );
                fields.push(quote! {pub #field_name: #enum_name});
                args.push(quote! {#field_name: #enum_name::new_from_tokens(&tokens[#idx..])});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
                param_types.push(quote! { types.push(ParamType::Enum(#enum_name::param_types())) });
            }
            _ => {
                let ty = expand_type(&param_type)?;

                let mut param_type_string = param_type.to_string();

                let param_type_string_ident_tok: proc_macro2::TokenStream =
                    param_type_string.parse().unwrap();

                param_types.push(quote! { types.push(ParamType::#param_type_string_ident_tok) });

                if let ParamType::Array(..) = param_type {
                    param_type_string = "Array".to_string();
                }
                if let ParamType::String(..) = param_type {
                    param_type_string = "String".to_string();
                }

                let param_type_string_ident = ident(&param_type_string);

                // Field declaration
                fields.push(quote! { pub #field_name: #ty});

                // `new_from_token()` instantiations
                let expected_str = format!(
                    "Failed to run `new_from_tokens()` for custom {} struct \
                (tokens have wrong order and/or wrong types)",
                    struct_name
                );
                args.push(quote! {
                    #field_name: <#ty>::from_token(tokens[#idx].clone()).expect(#expected_str)
                });

                // Token creation and insertion
                match param_type {
                    ParamType::Array(_t, _s) => {
                        struct_fields_tokens.push(
                            quote! {tokens.push(Token::#param_type_string_ident(vec![self.#field_name.into_token()]))},
                        );
                    }
                    // Primitive type
                    _ => {
                        // Token creation and insertion
                        struct_fields_tokens.push(
                            quote! {tokens.push(Token::#param_type_string_ident(self.#field_name))},
                        );
                    }
                }
            }
        }
    }

    // Actual creation of the struct, using the inner TokenStreams from above to produce the
    // TokenStream that represents the whole struct + methods declaration.
    Ok(quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct #struct_ident {
            #( #fields ),*
        }

        impl #struct_ident {
            pub fn param_types() -> Vec<ParamType> {
                let mut types = Vec::new();
                #( #param_types; )*
                types
            }

            pub fn into_token(self) -> Token {
                let mut tokens = Vec::new();
                #( #struct_fields_tokens; )*

                Token::Struct(tokens)
            }

            pub fn new_from_tokens(tokens: &[Token]) -> Self {
                Self {
                    #( #args ),*
                }
            }

        }

        impl fuels_core::Detokenize for #struct_ident {
            fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
                let token = match tokens.len() {
                    0 => Token::Struct(vec![]),
                    1 => tokens.remove(0),
                    _ => Token::Struct(tokens),
                };

                if let Token::Struct(tokens) = token.clone() {
                    Ok(#struct_ident::new_from_tokens(&tokens))
                } else {
                    Err(fuels_core::InvalidOutputType("Struct token doesn't contain inner tokens. This shouldn't happen.".to_string()))
                }
            }
        }
    })
}

/// Transforms a custom enum defined in [`Property`] into a [`TokenStream`]
/// that represents that same type as a Rust-native enum.
pub fn expand_custom_enum(name: &str, prop: &Property) -> Result<TokenStream, Error> {
    let components = prop.components.as_ref().unwrap();
    let mut enum_variants = Vec::with_capacity(components.len());

    // Holds a TokenStream representing the process of creating an enum [`Token`].
    let mut enum_selector_builder = Vec::new();

    // Holds the TokenStream representing the process of creating a Self enum from each `Token`.
    // Used when creating a struct from tokens with `MyEnum::new_from_tokens()`.
    let mut args = Vec::new();

    let enum_name = name.to_class_case();
    let enum_ident = ident(&enum_name);
    let mut param_types = Vec::new();

    for (discriminant, component) in components.iter().enumerate() {
        let variant_name = ident(&component.name.to_class_case());
        let dis = discriminant as u8;

        let param_type = parse_param(component)?;
        match param_type {
            // Case where an enum takes another enum
            ParamType::Enum(_params) => {
                // TODO: Support nested enums
                unimplemented!()
            }
            ParamType::Struct(_params) => {
                let inner_struct_name = &extract_custom_type_name_from_abi_property(
                    component,
                    Some(CustomType::Struct),
                )?
                .to_class_case();
                let inner_struct_ident = ident(inner_struct_name);
                // Enum variant declaration
                enum_variants.push(quote! { #variant_name(#inner_struct_ident)});

                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name(inner_struct) =>
                    (#dis, inner_struct.into_token())
                });

                // This is used for creating a new instance with `inner_struct::new_from_tokens()`
                // based on tokens received
                let expected_str = format!(
                    "Failed to run `new_from_tokens` for custom {} enum type",
                    enum_name
                );
                args.push(quote! {
                    (#dis, token) => {
                        let variant_content = <#inner_struct_ident>::from_tokens(vec![token]).expect(#expected_str);
                    #enum_ident::#variant_name(variant_content)
                        }
                });

                // This is used to get the correct nested types of the enum
                param_types.push(
                    quote! { types.push(ParamType::Struct(#inner_struct_ident::param_types()))
                    },
                );
            }
            // Unit type
            ParamType::Unit => {
                // Enum variant declaration
                enum_variants.push(quote! {#variant_name()});
                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name() => (#dis, Token::Unit)
                });
                param_types.push(quote! { types.push(ParamType::Unit) });
                args.push(quote! {(#dis, token) => #enum_ident::#variant_name(),});
            }
            // Elementary type
            _ => {
                let ty = expand_type(&param_type)?;
                let param_type_string = ident(&param_type.to_string());

                // Enum variant declaration
                enum_variants.push(quote! { #variant_name(#ty)});

                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name(value) => (#dis, Token::#param_type_string(value))
                });
                param_types.push(quote! { types.push(ParamType::#param_type_string) });
                args.push(
                    quote! {(#dis, token) => #enum_ident::#variant_name(<#ty>::from_tokens(vec![token])
                    .expect(&format!("Failed to run `new_from_tokens` for custom {} enum type",
                            #enum_name))),},
                );
            }
        }
    }

    // Actual creation of the enum, using the inner TokenStreams from above
    // to produce the TokenStream that represents the whole enum + methods
    // declaration.
    Ok(quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum #enum_ident {
            #( #enum_variants ),*
        }

        impl #enum_ident {
            pub fn param_types() -> Vec<ParamType> {
                let mut types = Vec::new();
                #( #param_types; )*
                types
            }

            pub fn into_token(self) -> Token {

                let (dis, tok) = match self {
                    #( #enum_selector_builder, )*
                };

                let selector = (dis, tok);
                Token::Enum(Box::new(selector))
            }

            pub fn new_from_tokens(tokens: &[Token]) -> Self {
                if tokens.is_empty() {
                    panic!("Empty tokens array received in `{}::new_from_tokens`",
                        #enum_name);
                }
                // For some reason sometimes we receive arrays that have multiple elements, with the
                // first token being a `Token::Enum`. We only consider that `Enum` token in that
                // case
                // TODO: figure out what is actually happening and if this is normal
                match tokens[0].clone() {
                    Token::Enum(content) => {
                        if let enum_selector = *content {
                            return match enum_selector {
                                #( #args )*
                                (_, _) => panic!("Failed to match with discriminant selector {:?}", enum_selector)
                            };
                        } else {
                            panic!("The EnumSelector `{:?}` didn't have a match", content);
                        }
                     },
                    _ => panic!("This should contain an `Enum` token, found `{:?}`", tokens),
                }
            }

        }

        impl fuels_core::Detokenize for #enum_ident {
            fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
                let token = match tokens.len() {
                    1 => tokens.remove(0),
                    _ => panic!("Received invalid number of tokens for creating {} enum (got {} expected 1)", #enum_name, tokens.len()),
                };
                if let Token::Enum(_) = token {
                    Ok(#enum_ident::new_from_tokens(&[token]))
                } else {
                    Err(fuels_core::InvalidOutputType("Enum token doesn't contain inner tokens."
                        .to_string()))
                }
            }
        }

    })
}

// A custom type name is coming in as `struct $name` or `enum $name`.
// We want to grab its `$name`.
pub fn extract_custom_type_name_from_abi_property(
    prop: &Property,
    expected: Option<CustomType>,
) -> Result<String, Error> {
    let type_field: Vec<&str> = prop.type_field.split_whitespace().collect();
    if type_field.len() != 2 {
        return Err(Error::MissingData(
            r#"The declared type was not in the format `{enum,struct} name`"#
                .parse()
                .unwrap(),
        ));
    }
    let (declared_type, type_name) = (type_field[0], type_field[1]);
    if let Some(expected_type) = expected {
        if expected_type.to_string() != declared_type {
            return Err(Error::InvalidType(format!(
                "Expected {} but {} was declared",
                expected_type.to_string(),
                declared_type
            )));
        }
    }
    Ok(String::from(type_name))
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
    use std::str::FromStr;

    #[test]
    fn test_extract_custom_type_name_from_abi_property_bad_data() {
        let p: Property = Default::default();
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
        assert!(matches!(result, Err(Error::MissingData(_))));
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("nowhitespacehere"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
        assert!(matches!(result, Err(Error::MissingData(_))));
    }

    #[test]
    fn test_extract_struct_name_from_abi_property_wrong_type() {
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("enum something"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
        assert!(matches!(result, Err(Error::InvalidType(_))));
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("struct somethingelse"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    fn test_extract_custom_type_name_from_abi_property() {
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("struct bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
        assert_eq!(result.unwrap(), "bar");
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("enum bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
        assert_eq!(result.unwrap(), "bar");
    }

    #[test]
    fn test_expand_custom_enum() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("unused"),
            components: Some(vec![
                Property {
                    name: String::from("long_island"),
                    type_field: String::from("u64"),
                    components: None,
                },
                Property {
                    name: String::from("moscow_mule"),
                    type_field: String::from("bool"),
                    components: None,
                },
            ]),
        };
        let result = expand_custom_enum("matcha_tea", &p);
        let expected = TokenStream::from_str(
            r#"
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchaTea {
    LongIsland(u64),
    MoscowMule(bool)
}
impl MatchaTea {
    pub fn param_types() -> Vec<ParamType> {
        let mut types = Vec::new();
        types.push(ParamType::U64);
        types.push(ParamType::Bool);
        types
    }
    pub fn into_token(self) -> Token {
        let (dis, tok) = match self {
            MatchaTea::LongIsland(value) => (0u8, Token::U64(value)),
            MatchaTea::MoscowMule(value) => (1u8, Token::Bool(value)),
        };
        let selector = (dis, tok);
        Token::Enum(Box::new(selector))
    }
    pub fn new_from_tokens(tokens: &[Token]) -> Self {
        if tokens.is_empty() {
            panic!("Empty tokens array received in `{}::new_from_tokens`", "MatchaTea");
        }
        match tokens[0].clone() {
            Token::Enum(content) => {
                if let enum_selector = *content {
                    return match enum_selector {
                        (0u8, token) => MatchaTea::LongIsland(
                            <u64> ::from_tokens(vec![token])
                                .expect(
                                &format!("Failed to run `new_from_tokens` for custom {} enum type",
                                "MatchaTea")
                                )
                        ),
                        (1u8, token) => MatchaTea::MoscowMule(
                            <bool> ::from_tokens(vec![token])
                                .expect(
                                &format!("Failed to run `new_from_tokens` for custom {} enum type",
                                "MatchaTea")
                                )
                        ),
                        (_, _) => panic!(
                            "Failed to match with discriminant selector {:?}",
                            enum_selector
                        )
                    };
                } else {
                    panic!("The EnumSelector `{:?}` didn't have a match", content);
                }
            },
            _ => panic!("This should contain an `Enum` token, found `{:?}`", tokens),
        }
    }
}
impl fuels_core::Detokenize for MatchaTea{
    fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
        let token = match tokens.len() {
            1 => tokens.remove(0),
            _ => panic!("Received invalid number of tokens for creating {} enum (got {} expected 1)", "MatchaTea", tokens.len()),
        };
        if let Token::Enum(_) = token {
            Ok(MatchaTea::new_from_tokens(&[token]))
        } else {
            Err(fuels_core::InvalidOutputType("Enum token doesn't contain inner tokens."
                .to_string()))
        }
    }
}
"#,
        );
        let expected = expected.unwrap().to_string();
        assert_eq!(result.unwrap().to_string(), expected);
    }

    #[test]
    fn test_expand_struct_inside_enum() {
        let inner_struct = Property {
            name: String::from("infrastructure"),
            type_field: String::from("struct Building"),
            components: Some(vec![
                Property {
                    name: String::from("rooms"),
                    type_field: String::from("u8"),
                    components: None,
                },
                Property {
                    name: String::from("floors"),
                    type_field: String::from("u16"),
                    components: None,
                },
            ]),
        };
        let enum_components = vec![
            inner_struct,
            Property {
                name: "service".to_string(),
                type_field: "u32".to_string(),
                components: None,
            },
        ];
        let p = Property {
            name: String::from("CityComponent"),
            type_field: String::from("enum CityComponent"),
            components: Some(enum_components),
        };
        let result = expand_custom_enum("Amsterdam", &p).unwrap();

        let expected = TokenStream::from_str(
            r#"
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Amsterdam {
    Infrastructure(Building),
    Service(u32)
}
impl Amsterdam {
    pub fn param_types() -> Vec<ParamType> {
        let mut types = Vec::new();
        types.push(ParamType::Struct(Building::param_types()));
        types.push(ParamType::U32);
        types
    }
    pub fn into_token(self) -> Token {
        let (dis, tok) = match self {
            Amsterdam::Infrastructure(inner_struct) => (0u8, inner_struct.into_token()),
            Amsterdam::Service(value) => (1u8, Token::U32(value)),
        };
        let selector = (dis, tok);
        Token::Enum(Box::new(selector))
    }
    pub fn new_from_tokens(tokens: &[Token]) -> Self {
        if tokens.is_empty() {
            panic!("Empty tokens array received in `{}::new_from_tokens`", "Amsterdam");
        }
        match tokens[0].clone() {
            Token::Enum(content) => {
                if let enum_selector = *content {
                    return match enum_selector {
                        (0u8, token) => {
                            let variant_content = <Building> ::from_tokens(vec![token]).expect(
                                "Failed to run `new_from_tokens` for custom Amsterdam enum type"
                            );
                            Amsterdam::Infrastructure(variant_content)
                        }
                        (1u8, token) => 
                            Amsterdam::Service(<u32> ::from_tokens(vec![token]).expect(&format!(
                                "Failed to run `new_from_tokens` for custom {} enum type",
                                "Amsterdam"
                            ))),
                        (_, _) => panic!(
                            "Failed to match with discriminant selector {:?}",
                            enum_selector
                        )
                    };
                } else {
                    panic!("The EnumSelector `{:?}` didn't have a match", content);
                }
            },
            _ => panic!("This should contain an `Enum` token, found `{:?}`", tokens),
        }
    }
}
impl fuels_core::Detokenize for Amsterdam{
    fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
        let token = match tokens.len() {
            1 => tokens.remove(0),
            _ => panic!("Received invalid number of tokens for creating {} enum (got {} expected 1)", "Amsterdam", tokens.len()),
        };
        if let Token::Enum(_) = token {
            Ok(Amsterdam::new_from_tokens(&[token]))
        } else {
            Err(fuels_core::InvalidOutputType("Enum token doesn't contain inner tokens."
                .to_string()))
        }
    }
}
            "#,
        )
        .unwrap();
        assert_eq!(result.to_string(), expected.to_string())
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    // Enum cannot contain enum at the moment
    fn test_expand_custom_enum_with_enum() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("unused"),
            components: Some(vec![Property {
                name: String::from("long_island"),
                type_field: String::from("enum cocktail"),
                components: Some(vec![Property {
                    name: String::from("cosmopolitan"),
                    type_field: String::from("bool"),
                    components: None,
                }]),
            }]),
        };
        let _ = expand_custom_enum("dragon", &p);
    }

    #[test]
    fn test_expand_custom_struct() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("struct cocktail"),
            components: Some(vec![
                Property {
                    name: String::from("long_island"),
                    type_field: String::from("bool"),
                    components: None,
                },
                Property {
                    name: String::from("cosmopolitan"),
                    type_field: String::from("u64"),
                    components: None,
                },
                Property {
                    name: String::from("mojito"),
                    type_field: String::from("u32"),
                    components: None,
                },
            ]),
        };
        let expected = TokenStream::from_str(
            r#"
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cocktail {
    pub long_island: bool,
    pub cosmopolitan: u64,
    pub mojito: u32
}
impl Cocktail {
    pub fn param_types() -> Vec<ParamType> {
        let mut types = Vec::new();
        types.push(ParamType::Bool);
        types.push(ParamType::U64);
        types.push(ParamType::U32);
        types
    }
    pub fn into_token(self) -> Token {
        let mut tokens = Vec::new();
        tokens.push(Token::Bool(self.long_island));
        tokens.push(Token::U64(self.cosmopolitan));
        tokens.push(Token::U32(self.mojito));
        Token::Struct(tokens)
    }
    pub fn new_from_tokens(tokens: &[Token]) -> Self {
        Self { 
        long_island : < bool > :: from_token (tokens [0usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom Cocktail struct (tokens have wrong order and/or wrong types)") , cosmopolitan : < u64 > :: from_token (tokens [1usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom Cocktail struct (tokens have wrong order and/or wrong types)") , mojito : < u32 > :: from_token (tokens [2usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom Cocktail struct (tokens have wrong order and/or wrong types)") }
    }
}
impl fuels_core::Detokenize for Cocktail {
    fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
        let token = match tokens.len() {
            0 => Token::Struct(vec![]),
            1 => tokens.remove(0),
            _ => Token::Struct(tokens),
        };
        if let Token::Struct(tokens) = token.clone() {
            Ok(Cocktail::new_from_tokens(&tokens))
        } else {
            Err(fuels_core::InvalidOutputType("Struct token doesn't contain inner tokens. This shouldn't happen.".to_string()))
        }
    }
}
        "#,
        );
        let expected = expected.unwrap().to_string();
        let result = expand_custom_struct(&p);
        assert_eq!(result.unwrap().to_string(), expected);
    }

    #[test]
    fn test_expand_custom_struct_with_struct() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("struct cocktail"),
            components: Some(vec![
                Property {
                    name: String::from("long_island"),
                    type_field: String::from("struct shaker"),
                    components: Some(vec![
                        Property {
                            name: String::from("cosmopolitan"),
                            type_field: String::from("bool"),
                            components: None,
                        },
                        Property {
                            name: String::from("bimbap"),
                            type_field: String::from("u64"),
                            components: None,
                        },
                    ]),
                },
                Property {
                    name: String::from("mojito"),
                    type_field: String::from("u32"),
                    components: None,
                },
            ]),
        };
        let expected = TokenStream::from_str(
            r#"
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cocktail {
    pub long_island: Shaker,
    pub mojito: u32
}
impl Cocktail {
    pub fn param_types() -> Vec<ParamType> {
        let mut types = Vec::new();
        types.push(ParamType::Struct(Shaker::param_types()));
        types.push(ParamType::U32);
        types
    }
    pub fn into_token(self) -> Token {
        let mut tokens = Vec::new();
        tokens.push(self.long_island.into_token());
        tokens.push(Token::U32(self.mojito));
        Token::Struct(tokens)
    }
    pub fn new_from_tokens(tokens: &[Token]) -> Self {
        Self { 
        long_island : Shaker :: new_from_tokens (& tokens [0usize ..]) , mojito : < u32 > :: from_token (tokens [1usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom Cocktail struct (tokens have wrong order and/or wrong types)") }
    }
}
impl fuels_core::Detokenize for Cocktail {
    fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
        let token = match tokens.len() {
            0 => Token::Struct(vec![]),
            1 => tokens.remove(0),
            _ => Token::Struct(tokens),
        };
        if let Token::Struct(tokens) = token.clone() {
            Ok(Cocktail::new_from_tokens(&tokens))
        } else {
            Err(fuels_core::InvalidOutputType("Struct token doesn't contain inner tokens. This shouldn't happen.".to_string()))
        }
    }
}
        "#,
        );
        let expected = expected.unwrap().to_string();
        let result = expand_custom_struct(&p);
        assert_eq!(result.unwrap().to_string(), expected);
    }
}
