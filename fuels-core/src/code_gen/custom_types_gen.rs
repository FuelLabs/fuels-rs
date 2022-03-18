use crate::errors::Error;
use crate::json_abi::parse_param;
use crate::types::expand_type;
use crate::utils::ident;
use crate::ParamType;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use strum_macros::ToString;
use sway_types::Property;

/// Functions used by the Abigen to expand custom types defined in an ABI spec.

#[derive(Debug, Clone, ToString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CustomType {
    Struct,
    Enum,
}

/// Transforms a custom type defined in [`Property`] into a [`TokenStream`]
/// that represents that same type as a Rust-native struct.
pub fn expand_internal_struct(prop: &Property) -> Result<TokenStream, Error> {
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
                let struct_name = ident(
                    &extract_custom_type_name_from_abi_property(component, &CustomType::Struct)?
                        .to_class_case(),
                );

                fields.push(quote! {pub #field_name: #struct_name});
                args.push(quote! {#field_name: #struct_name::new_from_tokens(&tokens[#idx..])});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
                param_types
                    .push(quote! { types.push(ParamType::Struct(#struct_name::param_types())) });
            }
            ParamType::Enum(_params) => {
                // TODO: Support enums inside structs
                unimplemented!()
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
                args.push(quote! {
                    #field_name: <#ty>::from_token(tokens[#idx].clone()).expect("Failed to run `new_from_tokens()` for custom struct, make sure to pass tokens in the right order and right types" )
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

    let name = ident(
        &extract_custom_type_name_from_abi_property(prop, &CustomType::Struct)?.to_class_case(),
    );

    // Actual creation of the struct, using the inner TokenStreams from above
    // to produce the TokenStream that represents the whole struct + methods
    // declaration.
    Ok(quote! {
        #[derive(Clone, Debug, Default, Eq, PartialEq)]
        pub struct #name {
            #( #fields ),*
        }

        impl #name {
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

        impl fuels_core::Detokenize for #name {
            fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, fuels_core::InvalidOutputType> {
                let token = match tokens.len() {
                    0 => Token::Struct(vec![]),
                    1 => tokens.remove(0),
                    _ => Token::Struct(tokens),
                };

                if let Token::Struct(tokens) = token.clone() {
                    Ok(#name::new_from_tokens(&tokens))
                } else {
                    Err(fuels_core::InvalidOutputType("Struct token doesn't contain inner tokens. This shouldn't happen.".to_string()))
                }
            }
        }
    })
}

/// Transforms a custom enum defined in [`Property`] into a [`TokenStream`]
/// that represents that same type as a Rust-native enum.
pub fn expand_internal_enum(name: &str, prop: &Property) -> Result<TokenStream, Error> {
    let components = prop.components.as_ref().unwrap();
    let mut fields = Vec::with_capacity(components.len());

    // Holds a TokenStream representing the process of
    // creating an enum [`Token`].
    let mut enum_selector_builder = Vec::new();

    let name = ident(&name.to_class_case());

    for (discriminant, component) in components.iter().enumerate() {
        let field_name = ident(&component.name.to_class_case());

        let param_type = parse_param(component)?;
        match param_type {
            // Case where an enum takes another enum
            ParamType::Enum(_params) => {
                // TODO: Support nested enums
                unimplemented!()
            }
            // Elementary type
            ParamType::Struct(_params) => {
                // TODO: Support structs inside enums
                unimplemented!()
            }
            _ => {
                let ty = expand_type(&param_type)?;
                let param_type_string = ident(&param_type.to_string());

                // Enum variant declaration
                fields.push(quote! { #field_name(#ty)});

                // Token creation
                enum_selector_builder.push(quote! {
                    #name::#field_name(value) => (#discriminant as u8, Token::#param_type_string(value))
                })
            }
        }
    }

    // Actual creation of the enum, using the inner TokenStreams from above
    // to produce the TokenStream that represents the whole enum + methods
    // declaration.
    Ok(quote! {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum #name {
            #( #fields ),*
        }

        impl #name {
            pub fn into_token(self) -> Token {

                let (dis, tok) = match self {
                    #( #enum_selector_builder, )*
                };

                let selector = (dis, tok);
                Token::Enum(Box::new(selector))
            }
        }
    })
}

// A custom type name is coming in as `struct $name` or `enum $name`.
// We want to grab its `$name`.
pub fn extract_custom_type_name_from_abi_property(
    prop: &Property,
    expected: &CustomType,
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
    if declared_type != expected.to_string() {
        return Err(Error::InvalidType(format!(
            "Expected {} but {} was declared",
            expected.to_string(),
            declared_type
        )));
    }
    Ok(String::from(type_name))
}

// Doing string -> TokenStream -> string isn't pretty but gives us the opportunity to
// have a better understanding of the generated code so we consider it ok.
// To generate the expected examples, output of the functions were taken
// with code @9ca376, and formatted in-IDE using rustfmt. It should be noted that
// rustfmt added an extra `,` after the last struct/enum field, which is not added
// by the `expand_internal_*` functions, and so was removed from the expected string.
// TODO(vnepveu): append extra `,` to last enum/struct field so it is aligned with rustfmt
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_extract_custom_type_name_from_abi_property_bad_data() {
        let p: Property = Default::default();
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Enum);
        assert!(matches!(result, Err(Error::MissingData(_))));
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("nowhitespacehere"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Enum);
        assert!(matches!(result, Err(Error::MissingData(_))));
    }

    #[test]
    fn test_extract_struct_name_from_abi_property_wrong_type() {
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("enum something"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Struct);
        assert!(matches!(result, Err(Error::InvalidType(_))));
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("struct somethingelse"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Enum);
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    fn test_extract_custom_type_name_from_abi_property() {
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("struct bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Struct);
        assert_eq!(result.unwrap(), "bar");
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("enum bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, &CustomType::Enum);
        assert_eq!(result.unwrap(), "bar");
    }

    #[test]
    fn test_expand_internal_enum() {
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
        let result = expand_internal_enum("matcha_tea", &p);
        let expected = TokenStream::from_str(
            r#"
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchaTea {
    LongIsland(u64),
    MoscowMule(bool)
}
impl MatchaTea {
    pub fn into_token(self) -> Token {
        let (dis, tok) = match self {
            MatchaTea::LongIsland(value) => (0usize as u8, Token::U64(value)),
            MatchaTea::MoscowMule(value) => (1usize as u8, Token::Bool(value)),
        };
        let selector = (dis, tok);
        Token::Enum(Box::new(selector))
    }
}
"#,
        );
        let expected = expected.unwrap().to_string();
        assert_eq!(result.unwrap().to_string(), expected);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    // Enum cannot contain struct at the moment
    fn test_expand_internal_enum_with_struct() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("unused"),
            components: Some(vec![Property {
                name: String::from("long_island"),
                type_field: String::from("struct cocktail"),
                components: Some(vec![Property {
                    name: String::from("cosmopolitan"),
                    type_field: String::from("bool"),
                    components: None,
                }]),
            }]),
        };
        let _ = expand_internal_enum("dragon", &p);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    // Enum cannot contain enum at the moment
    fn test_expand_internal_enum_with_enum() {
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
        let _ = expand_internal_enum("dragon", &p);
    }

    #[test]
    fn test_expand_internal_struct() {
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
        Self { long_island : < bool > :: from_token (tokens [0usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom struct, make sure to pass tokens in the right order and right types") , cosmopolitan : < u64 > :: from_token (tokens [1usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom struct, make sure to pass tokens in the right order and right types") , mojito : < u32 > :: from_token (tokens [2usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom struct, make sure to pass tokens in the right order and right types") }
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
        let result = expand_internal_struct(&p);
        assert_eq!(result.unwrap().to_string(), expected);
    }

    #[test]
    fn test_expand_internal_struct_with_struct() {
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
        Self { long_island : Shaker :: new_from_tokens (& tokens [0usize ..]) , mojito : < u32 > :: from_token (tokens [1usize] . clone ()) . expect ("Failed to run `new_from_tokens()` for custom struct, make sure to pass tokens in the right order and right types") }
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
        let result = expand_internal_struct(&p);
        assert_eq!(result.unwrap().to_string(), expected);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn test_expand_internal_struct_with_enum() {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("struct cocktail"),
            components: Some(vec![
                Property {
                    name: String::from("long_island"),
                    type_field: String::from("enum shaker"),
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
        let _ = expand_internal_struct(&p);
    }
}
