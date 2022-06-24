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
    let struct_name = &extract_custom_type_name_from_abi_property(prop, Some(CustomType::Struct))?;
    let struct_ident = ident(struct_name);
    let components = prop
        .components
        .as_ref()
        .expect("Fail to extract components from custom type");
    let mut fields = Vec::with_capacity(components.len());

    // Holds a TokenStream representing the process of
    // creating a [`Token`] and pushing it a vector of Tokens.
    let mut struct_fields_tokens = Vec::new();
    let mut param_types = Vec::new();

    // Holds the TokenStream representing the process
    // of creating a Self struct from each `Token`.
    // Used when creating a struct from tokens with
    // `Tokenizable::from_token()`.
    let mut args = Vec::new();

    // For each component, we create two TokenStreams:
    // 1. A struct field declaration like `pub #field_name: #component_name`
    // 2. The creation of a token and its insertion into a vector of Tokens.
    for component in components {
        let field_name = ident(&component.name.to_snake_case());
        let param_type = parse_param(component)?;

        match param_type {
            // Case where a struct takes another struct
            ParamType::Struct(_params) => {
                let inner_struct_ident = ident(&extract_custom_type_name_from_abi_property(
                    component,
                    Some(CustomType::Struct),
                )?);

                fields.push(quote! {pub #field_name: #inner_struct_ident});
                args.push(quote! {#field_name: #inner_struct_ident::from_token(next_token()?)?});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
                param_types.push(
                    quote! { types.push(ParamType::Struct(#inner_struct_ident::param_types())) },
                );
            }
            // The struct contains a nested enum
            ParamType::Enum(_params) => {
                let enum_name = ident(&extract_custom_type_name_from_abi_property(
                    component,
                    Some(CustomType::Enum),
                )?);
                fields.push(quote! {pub #field_name: #enum_name});
                args.push(quote! {#field_name: #enum_name::from_token(next_token()?)?});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });

                // The enum we're currently looking at must have variants due to
                // the usage of `EnumVariants`. Because of this we can safely
                // call unwrap() since this is the same ParamType::Enum that
                // will be used to generate the actual Enum type in Rust whose
                // param_types() method will not return an empty Vec due to the
                // aforementioned EnumVariants.
                let variants = quote! {EnumVariants::new(#enum_name::param_types()).unwrap()};

                param_types.push(quote! { types.push(ParamType::Enum(#variants)) });
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

                args.push(quote! {
                    #field_name: <#ty>::from_token(next_token()?)?
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

        impl Parameterize for #struct_ident {
            fn param_types() -> Vec<ParamType> {
                let mut types = Vec::new();
                #( #param_types; )*
                types
            }
        }

        impl Tokenizable for #struct_ident {
            fn into_token(self) -> Token {
                let mut tokens = Vec::new();
                #( #struct_fields_tokens; )*

                Token::Struct(tokens)
            }

            fn from_token(token: Token)  -> Result<Self, InstantiationError> {
                match token {
                    Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name)) })
                        };
                        Ok(Self { #( #args ),* })
                    },
                    other => Err(InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name, other))),
                }
            }
        }
    })
}

/// Transforms a custom enum defined in [`Property`] into a [`TokenStream`]
/// that represents that same type as a Rust-native enum.
pub fn expand_custom_enum(enum_name: &str, prop: &Property) -> Result<TokenStream, Error> {
    let components = prop
        .components
        .as_ref()
        .expect("Fail to extract components from custom type");
    let mut enum_variants = Vec::with_capacity(components.len());

    // Holds a TokenStream representing the process of creating an enum [`Token`].
    let mut enum_selector_builder = Vec::new();

    // Holds the TokenStream representing the process of creating a Self enum from each `Token`.
    // Used when creating a struct from tokens with `Tokenizable::from_token()`.
    let mut args = Vec::new();

    let enum_ident = ident(enum_name);
    let mut param_types = Vec::new();

    for (discriminant, component) in components.iter().enumerate() {
        let variant_name = ident(&component.name);
        let dis = discriminant as u8;

        let param_type = parse_param(component)?;
        match param_type {
            // Case where an enum takes another enum
            ParamType::Enum(_params) => {
                let inner_enum_name =
                    &extract_custom_type_name_from_abi_property(component, Some(CustomType::Enum))?;

                let inner_enum_ident = ident(inner_enum_name);
                // Enum variant declaration
                enum_variants.push(quote! { #variant_name(#inner_enum_ident)});

                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name(inner_enum) =>
                    (#dis, inner_enum.into_token())
                });

                args.push(quote! {
                    (#dis, token, _) => {
                        let variant_content = <#inner_enum_ident>::from_token(token)?;
                        Ok(#enum_ident::#variant_name(variant_content))
                    }
                });

                // For more info about this line check the enum inside struct
                let variants =
                    quote! {EnumVariants::new(#inner_enum_ident::param_types()).unwrap()};

                param_types.push(quote! { types.push(ParamType::Enum(#variants)) });
            }
            ParamType::Struct(_params) => {
                let inner_struct_name = &extract_custom_type_name_from_abi_property(
                    component,
                    Some(CustomType::Struct),
                )?;
                let inner_struct_ident = ident(inner_struct_name);
                // Enum variant declaration
                enum_variants.push(quote! { #variant_name(#inner_struct_ident)});

                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name(inner_struct) =>
                    (#dis, inner_struct.into_token())
                });

                args.push(quote! {
                    (#dis, token, _) => {
                        let variant_content = <#inner_struct_ident>::from_token(token)?;
                        Ok(#enum_ident::#variant_name(variant_content))
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
                args.push(quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name()),});
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
                    quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name(<#ty>::from_token(token)?)),},
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

        impl Parameterize for #enum_ident {
            fn param_types() -> Vec<ParamType> {
                let mut types = Vec::new();
                #( #param_types; )*
                types
            }
        }

        impl Tokenizable for #enum_ident {
            fn into_token(self) -> Token {
                let (dis, tok) = match self {
                    #( #enum_selector_builder, )*
                };

                let variants = EnumVariants::new(Self::param_types()).unwrap();
                let selector = (dis, tok, variants);
                Token::Enum(Box::new(selector))
            }

            fn from_token(token: Token)  -> Result<Self, InstantiationError> {
                if let Token::Enum(enum_selector) = token {
                        match *enum_selector {
                            #( #args )*
                            (_, _, _) => Err(InstantiationError(format!("Could not construct '{}'. Failed to match with discriminant selector {:?}", #enum_name, enum_selector)))
                        }
                }
                else {
                    Err(InstantiationError(format!("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}", #enum_name, token)))
                }
            }
        }
    })
}

// A custom type name should be passed to this function as `{struct,enum} $name`,
// or inside an array, like `[{struct,enum} $name; $length]`.
// This function extracts the `$name`.
pub fn extract_custom_type_name_from_abi_property(
    prop: &Property,
    expected: Option<CustomType>,
) -> Result<String, Error> {
    let type_field = match prop.type_field.starts_with('[') && prop.type_field.ends_with(']') {
        // Check for custom type inside array.
        true => {
            // Split `[struct | enum $name; $length]` into `[struct | enum $name` and ` $length]`
            let type_field = prop.type_field.split(';').collect::<Vec<&str>>()[0]
                .chars()
                .skip(1) // Remove `[` from `[struct | enum $name`.
                .collect::<String>(); // Return `struct | enum $name`.

            type_field
        }
        // If it's not inside an array, return the `{struct,enum} $name`.
        false => prop.type_field.clone(),
    };

    // Split `{struct,enum} $name` into `{struct,enum}` and `$name`.
    let type_field: Vec<&str> = type_field.split_whitespace().collect();

    if type_field.len() != 2 {
        return Err(Error::MissingData(
            r#"The declared type was not in the format `{enum,struct} name`"#
                .parse()
                .unwrap(),
        ));
    };

    if let Some(expected_type) = expected {
        if expected_type.to_string() != type_field[0] {
            return Err(Error::InvalidType(format!(
                "Expected {} but {} was declared",
                expected_type.to_string(),
                type_field[0]
            )));
        }
    }

    // Return the `$name`.
    Ok(type_field[1].to_string())
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
    fn test_extract_custom_type_name_from_abi_property() -> Result<(), Error> {
        let p = Property {
            name: String::from("foo"),
            type_field: String::from("struct bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
        assert_eq!(result?, "bar");

        let p = Property {
            name: String::from("foo"),
            type_field: String::from("enum bar"),
            components: None,
        };
        let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
        assert_eq!(result?, "bar");
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum() -> Result<(), Error> {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("unused"),
            components: Some(vec![
                Property {
                    name: String::from("LongIsland"),
                    type_field: String::from("u64"),
                    components: None,
                },
                Property {
                    name: String::from("MoscowMule"),
                    type_field: String::from("bool"),
                    components: None,
                },
            ]),
        };
        let actual = expand_custom_enum("MatchaTea", &p)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum MatchaTea { LongIsland (u64) , MoscowMule (bool) } impl Parameterize for MatchaTea { fn param_types () -> Vec < ParamType > { let mut types = Vec :: new () ; types . push (ParamType :: U64) ; types . push (ParamType :: Bool) ; types } } impl Tokenizable for MatchaTea { fn into_token (self) -> Token { let (dis , tok) = match self { MatchaTea :: LongIsland (value) => (0u8 , Token :: U64 (value)) , MatchaTea :: MoscowMule (value) => (1u8 , Token :: Bool (value)) , } ; let variants = EnumVariants :: new (Self :: param_types ()) . unwrap () ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , InstantiationError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => Ok (MatchaTea :: LongIsland (< u64 > :: from_token (token) ?)) , (1u8 , token , _) => Ok (MatchaTea :: MoscowMule (< bool > :: from_token (token) ?)) , (_ , _ , _) => Err (InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "MatchaTea" , enum_selector))) } } else { Err (InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "MatchaTea" , token))) } } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_struct_inside_enum() -> Result<(), Error> {
        let inner_struct = Property {
            name: String::from("Infrastructure"),
            type_field: String::from("struct Building"),
            components: Some(vec![
                Property {
                    name: String::from("Rooms"),
                    type_field: String::from("u8"),
                    components: None,
                },
                Property {
                    name: String::from("Floors"),
                    type_field: String::from("u16"),
                    components: None,
                },
            ]),
        };
        let enum_components = vec![
            inner_struct,
            Property {
                name: "Service".to_string(),
                type_field: "u32".to_string(),
                components: None,
            },
        ];
        let p = Property {
            name: String::from("CityComponent"),
            type_field: String::from("enum CityComponent"),
            components: Some(enum_components),
        };
        let actual = expand_custom_enum("Amsterdam", &p)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum Amsterdam { Infrastructure (Building) , Service (u32) } impl Parameterize for Amsterdam { fn param_types () -> Vec < ParamType > { let mut types = Vec :: new () ; types . push (ParamType :: Struct (Building :: param_types ())) ; types . push (ParamType :: U32) ; types } } impl Tokenizable for Amsterdam { fn into_token (self) -> Token { let (dis , tok) = match self { Amsterdam :: Infrastructure (inner_struct) => (0u8 , inner_struct . into_token ()) , Amsterdam :: Service (value) => (1u8 , Token :: U32 (value)) , } ; let variants = EnumVariants :: new (Self :: param_types ()) . unwrap () ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , InstantiationError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < Building > :: from_token (token) ? ; Ok (Amsterdam :: Infrastructure (variant_content)) } (1u8 , token , _) => Ok (Amsterdam :: Service (< u32 > :: from_token (token) ?)) , (_ , _ , _) => Err (InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "Amsterdam" , enum_selector))) } } else { Err (InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "Amsterdam" , token))) } } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum_with_enum() -> Result<(), Error> {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("unused"),
            components: Some(vec![Property {
                name: String::from("El2"),
                type_field: String::from("enum EnumLevel2"),
                components: Some(vec![Property {
                    name: String::from("El1"),
                    type_field: String::from("enum EnumLevel1"),
                    components: Some(vec![Property {
                        name: String::from("Num"),
                        type_field: String::from("u32"),
                        components: None,
                    }]),
                }]),
            }]),
        };
        let actual = expand_custom_enum("EnumLevel3", &p)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum EnumLevel3 { El2 (EnumLevel2) } impl Parameterize for EnumLevel3 { fn param_types () -> Vec < ParamType > { let mut types = Vec :: new () ; types . push (ParamType :: Enum (EnumVariants :: new (EnumLevel2 :: param_types ()) . unwrap ())) ; types } } impl Tokenizable for EnumLevel3 { fn into_token (self) -> Token { let (dis , tok) = match self { EnumLevel3 :: El2 (inner_enum) => (0u8 , inner_enum . into_token ()) , } ; let variants = EnumVariants :: new (Self :: param_types ()) . unwrap () ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , InstantiationError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < EnumLevel2 > :: from_token (token) ? ; Ok (EnumLevel3 :: El2 (variant_content)) } (_ , _ , _) => Err (InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "EnumLevel3" , enum_selector))) } } else { Err (InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "EnumLevel3" , token))) } } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct() -> Result<(), Error> {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("struct Cocktail"),
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
        let actual = expand_custom_struct(&p)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : bool , pub cosmopolitan : u64 , pub mojito : u32 } impl Parameterize for Cocktail { fn param_types () -> Vec < ParamType > { let mut types = Vec :: new () ; types . push (ParamType :: Bool) ; types . push (ParamType :: U64) ; types . push (ParamType :: U32) ; types } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: Bool (self . long_island)) ; tokens . push (Token :: U64 (self . cosmopolitan)) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , InstantiationError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : < bool > :: from_token (next_token () ?) ? , cosmopolitan : < u64 > :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct_with_struct() -> Result<(), Error> {
        let p = Property {
            name: String::from("unused"),
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                Property {
                    name: String::from("long_island"),
                    type_field: String::from("struct Shaker"),
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
        let actual = expand_custom_struct(&p)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : Shaker , pub mojito : u32 } impl Parameterize for Cocktail { fn param_types () -> Vec < ParamType > { let mut types = Vec :: new () ; types . push (ParamType :: Struct (Shaker :: param_types ())) ; types . push (ParamType :: U32) ; types } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (self . long_island . into_token ()) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , InstantiationError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : Shaker :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }
}
