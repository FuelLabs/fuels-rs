use crate::code_gen::custom_types::{param_type_calls, Component};
use crate::code_gen::docs_gen::expand_doc;
use crate::code_gen::resolved_type;
use crate::utils::safe_ident;
use fuels_types::errors::Error;
use fuels_types::{ABIFunction, TypeDeclaration};
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use resolved_type::resolve_type;
use std::collections::HashMap;

/// Functions used by the Abigen to expand functions defined in an ABI spec.

/// Transforms a function defined in [`Function`] into a [`TokenStream`]
/// that represents that same function signature as a Rust-native function
/// declaration.
/// The actual logic inside the function is the function `method_hash` under
/// [`Contract`], which is responsible for encoding the function selector
/// and the function parameters that will be used in the actual contract call.
///
/// [`Contract`]: crate::contract::Contract
pub fn expand_function(
    function: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    if function.name.is_empty() {
        return Err(Error::InvalidData("Function name can not be empty".into()));
    }

    let args = function_arguments(function, types)?;

    let arg_names = args.iter().map(|component| &component.field_name);

    let param_type_calls = param_type_calls(&args);

    let arg_declarations = args.iter().map(|component| {
        let name = &component.field_name;
        let field_type: TokenStream = (&component.field_type).into();
        quote! { #name: #field_type }
    });

    let doc = expand_doc(&format!(
        "Calls the contract's `{}` function",
        function.name,
    ));

    let name = safe_ident(&function.name);
    let name_stringified = name.to_string();

    let output_type = resolve_fn_output_type(function, types)?;

    Ok(quote! {
        #doc
        pub fn #name(&self #(,#arg_declarations)*) -> ContractCallHandler<#output_type> {
            let provider = self.wallet.get_provider().expect("Provider not set up");
            let encoded_fn_selector = resolve_fn_selector(#name_stringified, &[#(#param_type_calls),*]);
            let tokens = [#(#arg_names.into_token()),*];
            Contract::method_hash(&provider,
                self.contract_id.clone(),
                &self.wallet,
                encoded_fn_selector,
                &tokens).expect("method not found (this should never happen)")
        }
    })
}

fn resolve_fn_output_type(
    function: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let output_type = resolve_type(&function.output, types)?;
    if output_type.uses_vectors() {
        Err(Error::CompilationError(format!(
            "function '{}' contains a vector in its return type. This currently isn't supported.",
            function.name
        )))
    } else {
        Ok(output_type.into())
    }
}

fn function_arguments(
    fun: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<Vec<Component>, Error> {
    fun.inputs
        .iter()
        .map(|input| Component::new(input, types, true))
        .collect::<Result<Vec<_>, anyhow::Error>>()
        .map_err(|e| Error::InvalidType(e.to_string()))
}

/// Expands a positional identifier string that may be empty.
/// Note that this expands the parameter name with `safe_ident`, meaning that
/// identifiers that are reserved keywords get `_` appended to them.
pub fn expand_input_name(name: &str) -> Result<TokenStream, Error> {
    if name.is_empty() {
        return Err(Error::InvalidData(
            "Function arguments can not have empty names".into(),
        ));
    }
    let name = safe_ident(&name.to_snake_case());
    Ok(quote! { #name })
}

// Regarding string->TokenStream->string, refer to `custom_types` tests for more details.
#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::{ProgramABI, TypeApplication};
    use std::str::FromStr;

    #[test]
    fn test_expand_function_simpleabi() -> Result<(), Error> {
        let s = r#"
            {
                "types": [
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 10,
                    "type": "bool",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 12,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 2,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 3,
                    "type": "struct MyStruct2",
                    "components": [
                      {
                        "name": "x",
                        "type": 10,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 12,
                        "typeArguments": []
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 26,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  }
                ],
                "functions": [
                  {
                    "type": "function",
                    "inputs": [
                      {
                        "name": "s1",
                        "type": 2,
                        "typeArguments": []
                      },
                      {
                        "name": "s2",
                        "type": 3,
                        "typeArguments": []
                      }
                    ],
                    "name": "some_abi_funct",
                    "output": {
                      "name": "",
                      "type": 26,
                      "typeArguments": []
                    }
                  }
                ]
              }
    "#;
        let parsed_abi: ProgramABI = serde_json::from_str(s)?;
        let all_types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        // Grabbing the one and only function in it.
        let result = expand_function(&parsed_abi.functions[0], &all_types)?;

        let expected_code = r#"
                #[doc = "Calls the contract's `some_abi_funct` function"]
                pub fn some_abi_funct(&self, s_1: MyStruct1, s_2: MyStruct2) -> ContractCallHandler<MyStruct1> {
                    let provider = self.wallet.get_provider().expect("Provider not set up");
                    let encoded_fn_selector = resolve_fn_selector(
                        "some_abi_funct",
                        &[<MyStruct1> :: param_type(), <MyStruct2> :: param_type()]
                    );
                    let tokens = [s_1.into_token(), s_2.into_token()];
                    Contract::method_hash(
                        &provider,
                        self.contract_id.clone(),
                        &self.wallet,
                        encoded_fn_selector,
                        &tokens
                    )
                    .expect("method not found (this should never happen)")
                }
        "#;

        let expected = TokenStream::from_str(expected_code).unwrap().to_string();

        assert_eq!(result.to_string(), expected);

        Ok(())
    }

    #[test]
    fn test_expand_function_simple() -> Result<(), Error> {
        let the_function = ABIFunction {
            inputs: vec![TypeApplication {
                name: String::from("bimbam"),
                type_id: 1,
                ..Default::default()
            }],
            name: "HelloWorld".to_string(),
            ..Default::default()
        };
        let types = [
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: String::from("()"),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = expand_function(&the_function, &types);
        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `HelloWorld` function"]
            pub fn HelloWorld(&self, bimbam: bool) -> ContractCallHandler<()> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let encoded_fn_selector = resolve_fn_selector("HelloWorld", &[<bool> :: param_type()]);
                let tokens = [bimbam.into_token()];
                Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    encoded_fn_selector,
                    &tokens
                )
                .expect("method not found (this should never happen)")
            }
            "#,
        );
        let expected = expected?.to_string();

        assert_eq!(result?.to_string(), expected);
        Ok(())
    }

    #[test]
    fn test_expand_function_complex() -> Result<(), Error> {
        let the_function = ABIFunction {
            inputs: vec![TypeApplication {
                name: String::from("the_only_allowed_input"),
                type_id: 4,
                ..Default::default()
            }],
            name: "hello_world".to_string(),
            output: TypeApplication {
                name: String::from("stillnotused"),
                type_id: 1,
                ..Default::default()
            },
        };
        let types = [
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("enum EntropyCirclesEnum"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("Postcard"),
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("Teacup"),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                4,
                TypeDeclaration {
                    type_id: 4,
                    type_field: String::from("struct SomeWeirdFrenchCuisine"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("Beef"),
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("BurgundyWine"),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = expand_function(&the_function, &types);
        // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `hello_world` function"]
            pub fn hello_world(
                &self,
                the_only_allowed_input: SomeWeirdFrenchCuisine
            ) -> ContractCallHandler<EntropyCirclesEnum> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let encoded_fn_selector = resolve_fn_selector("hello_world", &[<SomeWeirdFrenchCuisine> :: param_type()]);
                let tokens = [the_only_allowed_input.into_token()];
                Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    encoded_fn_selector,
                    &tokens
                )
                .expect("method not found (this should never happen)")
            }
            "#,
        );
        let expected = expected?.to_string();

        assert_eq!(result?.to_string(), expected);
        Ok(())
    }

    // --- expand_selector ---

    // // --- expand_function_argument ---
    #[test]
    fn test_expand_function_arguments() -> Result<(), Error> {
        let the_argument = TypeApplication {
            name: "some_argument".to_string(),
            type_id: 0,
            ..Default::default()
        };

        // All arguments are here
        let the_function = ABIFunction {
            inputs: vec![the_argument],
            ..ABIFunction::default()
        };

        let types = [(
            0,
            TypeDeclaration {
                type_id: 0,
                type_field: String::from("u32"),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(&the_function, &types)?;
        let component = &result[0];

        assert_eq!(&component.field_name.to_string(), "some_argument");
        assert_eq!(&component.field_type.to_string(), "u32");

        Ok(())
    }

    #[test]
    fn test_expand_function_arguments_primitive() -> Result<(), Error> {
        let the_function = ABIFunction {
            inputs: vec![TypeApplication {
                name: "bim_bam".to_string(),
                type_id: 1,
                ..Default::default()
            }],
            name: "pip_pop".to_string(),
            ..Default::default()
        };

        let types = [
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: String::from("()"),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(&the_function, &types)?;
        let component = &result[0];

        assert_eq!(&component.field_name.to_string(), "bim_bam");
        assert_eq!(&component.field_type.to_string(), "u64");

        Ok(())
    }

    #[test]
    fn test_expand_function_arguments_composite() -> Result<(), Error> {
        let mut function = ABIFunction {
            inputs: vec![TypeApplication {
                name: "bim_bam".to_string(),
                type_id: 0,
                ..Default::default()
            }],
            name: "PipPopFunction".to_string(),
            ..Default::default()
        };

        let types = [
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: "struct CarMaker".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "name".to_string(),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: "str[5]".to_string(),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: "enum Cocktail".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "variant".to_string(),
                        type_id: 3,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: "u32".to_string(),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(&function, &types)?;
        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "CarMaker");

        function.inputs[0].type_id = 2;
        let result = function_arguments(&function, &types)?;
        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "Cocktail");

        Ok(())
    }

    #[test]
    fn transform_name_to_snake_case() -> Result<(), Error> {
        let result = expand_input_name("CamelCaseHello");
        assert_eq!(result?.to_string(), "camel_case_hello");
        Ok(())
    }

    #[test]
    fn avoids_collisions_with_keywords() -> Result<(), Error> {
        let result = expand_input_name("if");
        assert_eq!(result?.to_string(), "if_");

        let result = expand_input_name("let");
        assert_eq!(result?.to_string(), "let_");
        Ok(())
    }
}
