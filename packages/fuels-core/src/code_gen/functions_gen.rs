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

    let output_type: TokenStream = resolve_type(&function.output, types)?.into();

    let name = safe_ident(&function.name);
    let name_stringified = name.to_string();
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
    use fuels_types::ProgramABI;
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

    // TODO: Move tests using the old abigen to the new one.
    // Currently, they will be skipped. Even though we're not fully testing these at
    // unit level, they're tested at integration level, in the main harness.rs file.

    // #[test]
    // fn test_expand_function_simple() -> Result<(), Error> {
    //     let mut the_function = Function {
    //         type_field: "unused".to_string(),
    //         inputs: vec![],
    //         name: "HelloWorld".to_string(),
    //         outputs: vec![],
    //     };
    //     the_function.inputs.push(Property {
    //         name: String::from("bimbam"),
    //         type_field: String::from("bool"),
    //         components: None,
    //     });
    //     let result = expand_function(&the_function, &Default::default(), &Default::default());
    //     let expected = TokenStream::from_str(
    //         r#"
    //         #[doc = "Calls the contract's `HelloWorld` (0x0000000097d4de45) function"]
    //         pub fn HelloWorld(&self, bimbam: bool) -> ContractCallHandler<()> {
    //             Contract::method_hash(
    //                 &self.wallet.get_provider().expect("Provider not set up"),
    //                 self.contract_id.clone(),
    //                 &self.wallet,
    //                 [0, 0, 0, 0, 151, 212, 222, 69],
    //                 None,
    //                 &[bimbam.into_token() ,]
    //             )
    //             .expect("method not found (this should never happen)")
    //         }
    //         "#,
    //     );
    //     let expected = expected?.to_string();

    //     assert_eq!(result?.to_string(), expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_complex() -> Result<(), Error> {
    //     let mut the_function = Function {
    //         type_field: "function".to_string(),
    //         name: "hello_world".to_string(),
    //         inputs: vec![],
    //         outputs: vec![Property {
    //             name: String::from("stillnotused"),
    //             type_field: String::from("enum EntropyCirclesEnum"),
    //             components: Some(vec![
    //                 Property {
    //                     name: String::from("Postcard"),
    //                     type_field: String::from("bool"),
    //                     components: None,
    //                 },
    //                 Property {
    //                     name: String::from("Teacup"),
    //                     type_field: String::from("u64"),
    //                     components: None,
    //                 },
    //             ]),
    //         }],
    //     };
    //     the_function.inputs.push(Property {
    //         name: String::from("the_only_allowed_input"),
    //         type_field: String::from("struct BurgundyBeefStruct"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("Beef"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("BurgundyWine"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //         ]),
    //     });
    //     let mut custom_structs = HashMap::new();
    //     custom_structs.insert(
    //         "BurgundyBeefStruct".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct SomeWeirdFrenchCuisine".to_string(),
    //             components: None,
    //         },
    //     );
    //     custom_structs.insert(
    //         "CoolIndieGame".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct CoolIndieGame".to_string(),
    //             components: None,
    //         },
    //     );
    //     let mut custom_enums = HashMap::new();
    //     custom_enums.insert(
    //         "EntropyCirclesEnum".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "enum EntropyCirclesEnum".to_string(),
    //             components: None,
    //         },
    //     );
    //     let result = expand_function(&the_function, &custom_enums, &custom_structs);
    //     // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
    //     let expected = TokenStream::from_str(
    //         r#"
    //         #[doc = "Calls the contract's `hello_world` (0x0000000076b25a24) function"]
    //         pub fn hello_world(
    //             &self,
    //             the_only_allowed_input: SomeWeirdFrenchCuisine
    //         ) -> ContractCallHandler<EntropyCirclesEnum> {
    //             Contract::method_hash(
    //                 &self.wallet.get_provider().expect("Provider not set up"),
    //                 self.contract_id.clone(),
    //                 &self.wallet,
    //                 [0, 0, 0, 0, 118, 178, 90, 36],
    //                 Some(ParamType::Enum(EnumVariants::new(vec![ParamType::Bool, ParamType::U64]).unwrap())),
    //                 &[the_only_allowed_input.into_token() ,]
    //             )
    //             .expect("method not found (this should never happen)")
    //         }
    //         "#,
    //     );
    //     let expected = expected?.to_string();

    //     assert_eq!(result?.to_string(), expected);
    //     Ok(())
    // }

    // --- expand_selector ---

    // --- expand_fn_outputs ---
    // #[test]
    // fn test_expand_fn_outputs() -> Result<(), Error> {
    //     let result = expand_fn_outputs(&[]);
    //     assert_eq!(result?.to_string(), "()");

    //     // Primitive type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: "bool".to_string(),
    //         components: None,
    //     }]);
    //     assert_eq!(result?.to_string(), "bool");

    //     // Struct type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: String::from("struct streaming_services"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("thistypedoesntexist"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("thistypedoesntexist"),
    //                 components: None,
    //             },
    //         ]),
    //     }]);
    //     assert_eq!(result?.to_string(), "streaming_services");

    //     // Enum type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: String::from("enum StreamingServices"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //         ]),
    //     }]);
    //     assert_eq!(result?.to_string(), "StreamingServices");
    //     Ok(())
    // }

    // // --- expand_function_argument ---
    // #[test]
    // fn test_expand_function_arguments() -> Result<(), Error> {
    //     let hm: HashMap<String, Property> = HashMap::new();
    //     let the_argument = Property {
    //         name: "some_argument".to_string(),
    //         type_field: String::from("u32"),
    //         components: None,
    //     };

    //     // All arguments are here
    //     let mut the_function = Function {
    //         type_field: "".to_string(),
    //         inputs: vec![],
    //         name: "".to_string(),
    //         outputs: vec![],
    //     };
    //     the_function.inputs.push(the_argument);

    //     let result = expand_function_arguments(&the_function, &hm, &hm);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = "(, some_argument : u32,& [some_argument . into_token () ,])";

    //     assert_eq!(result, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_arguments_primitive() -> Result<(), Error> {
    //     let hm: HashMap<String, Property> = HashMap::new();
    //     let mut the_function = Function {
    //         type_field: "function".to_string(),
    //         inputs: vec![],
    //         name: "pip_pop".to_string(),
    //         outputs: vec![],
    //     };

    //     the_function.inputs.push(Property {
    //         name: "bim_bam".to_string(),
    //         type_field: String::from("u64"),
    //         components: None,
    //     });
    //     let result = expand_function_arguments(&the_function, &hm, &hm);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);

    //     assert_eq!(result, "(, bim_bam : u64,& [bim_bam . into_token () ,])");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_arguments_composite() -> Result<(), Error> {
    //     let mut function = Function {
    //         type_field: "zig_zag".to_string(),
    //         inputs: vec![],
    //         name: "PipPopFunction".to_string(),
    //         outputs: vec![],
    //     };
    //     function.inputs.push(Property {
    //         name: "bim_bam".to_string(),
    //         type_field: String::from("struct CarMaker"),
    //         components: Some(vec![Property {
    //             name: "name".to_string(),
    //             type_field: "str[5]".to_string(),
    //             components: None,
    //         }]),
    //     });
    //     let mut custom_structs = HashMap::new();
    //     custom_structs.insert(
    //         "CarMaker".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct CarMaker".to_string(),
    //             components: None,
    //         },
    //     );
    //     let mut custom_enums = HashMap::new();
    //     custom_enums.insert(
    //         "Cocktail".to_string(),
    //         Property {
    //             name: "Cocktail".to_string(),
    //             type_field: "enum Cocktail".to_string(),
    //             components: Some(vec![Property {
    //                 name: "variant".to_string(),
    //                 type_field: "u32".to_string(),
    //                 components: None,
    //             }]),
    //         },
    //     );

    //     let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = r#"(, bim_bam : CarMaker,& [bim_bam . into_token () ,])"#;
    //     assert_eq!(result, expected);

    //     function.inputs[0].type_field = "enum Cocktail".to_string();
    //     let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = r#"(, bim_bam : Cocktail,& [bim_bam . into_token () ,])"#;
    //     assert_eq!(result, expected);
    //     Ok(())
    // }

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

    // --- expand_input_param ---
    // #[test]
    // fn test_expand_input_param_primitive() -> Result<(), Error> {
    //     let def = Function::default();
    //     let result = expand_input_param(&def, "unused", &ParamType::Bool, &None);
    //     assert_eq!(result?.to_string(), "bool");

    //     let result = expand_input_param(&def, "unused", &ParamType::U64, &None);
    //     assert_eq!(result?.to_string(), "u64");

    //     let result = expand_input_param(&def, "unused", &ParamType::String(10), &None);
    //     assert_eq!(result?.to_string(), "String");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_array() -> Result<(), Error> {
    //     let array_type = ParamType::Array(Box::new(ParamType::U64), 10);
    //     let result = expand_input_param(&Function::default(), "unused", &array_type, &None);
    //     assert_eq!(result?.to_string(), ":: std :: vec :: Vec < u64 >");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_custom_type() -> Result<(), Error> {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Babies"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert_eq!(result?.to_string(), "Babies");

    //     let enum_type = ParamType::Enum(EnumVariants::new(vec![ParamType::U8, ParamType::U32])?);
    //     let enum_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("enum Babies"),
    //         components: None,
    //     };
    //     let enum_name = Some(&enum_prop);
    //     let result = expand_input_param(&def, "unused", &enum_type, &enum_name);
    //     assert_eq!(result?.to_string(), "Babies");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_struct_wrong_name() {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("not_the_right_format"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert!(matches!(result, Err(Error::InvalidData(_))));
    // }

    // #[test]
    // fn test_expand_input_param_struct_with_enum_name() {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("enum Butitsastruct"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert!(matches!(result, Err(Error::InvalidType(_))));
    // }

    // #[test]
    // fn can_have_b256_mixed_in_tuple_w_custom_types() -> anyhow::Result<()> {
    //     let test_struct_component = Property {
    //         name: "__tuple_element".to_string(),
    //         type_field: "struct TestStruct".to_string(),
    //         components: Some(vec![Property {
    //             name: "value".to_string(),
    //             type_field: "u64".to_string(),
    //             components: None,
    //         }]),
    //     };
    //     let b256_component = Property {
    //         name: "__tuple_element".to_string(),
    //         type_field: "b256".to_string(),
    //         components: None,
    //     };

    //     let property = Property {
    //         name: "".to_string(),
    //         type_field: "(struct TestStruct, b256)".to_string(),
    //         components: Some(vec![test_struct_component, b256_component]),
    //     };

    //     let stream = expand_fn_outputs(slice::from_ref(&property))?;

    //     let actual = stream.to_string();
    //     let expected = "(TestStruct , [u8 ; 32])";

    //     assert_eq!(actual, expected);

    //     Ok(())
    // }
}
