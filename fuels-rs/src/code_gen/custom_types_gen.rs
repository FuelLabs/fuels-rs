use crate::errors::Error;
use crate::json_abi::parse_param;
use crate::types::expand_type;
use crate::utils::ident;
use core_types::Property;
use fuels_core::ParamType;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;

/// Functions used by the Abigen to expand custom types defined in an ABI spec.

// TODO: Right now we have an "end-to-end" test suite for the Abigen!
// under `fuels-abigen/tests/harness.rs`. But it would be nice to have
// tests at the level of this component.

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
                let struct_name =
                    ident(&extract_struct_name_from_abi_property(component).to_class_case());

                fields.push(quote! {pub #field_name: #struct_name});
                args.push(quote! {#field_name: #struct_name::new_from_tokens(&tokens[#idx..])});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
                param_types
                    .push(quote! { types.push(ParamType::Struct(#struct_name::param_types())) });
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

    let name = ident(&extract_struct_name_from_abi_property(prop).to_class_case());

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

                Ok(#name::new_from_tokens(&[token]))
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

// A custom type name is coming in as `struct $name
// We want to grab its `$name`.
pub fn extract_struct_name_from_abi_property(prop: &Property) -> String {
    prop.type_field.split_whitespace().collect::<Vec<&str>>()[1].to_string()
}
