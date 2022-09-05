use crate::code_gen::custom_types_gen::extract_custom_type_name_from_abi_property;
use crate::code_gen::docs_gen::expand_doc;
use crate::types::expand_type;
use crate::utils::{first_four_bytes_of_sha256_hash, ident, safe_ident};
use crate::{ParamType, Selector};
use fuels_types::errors::Error;
use fuels_types::function_selector::build_fn_selector;
use fuels_types::{
    ABIFunction, CustomType, TypeApplication, TypeDeclaration, ENUM_KEYWORD, STRUCT_KEYWORD,
};
use inflector::Inflector;
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use regex::Regex;
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

    let fn_param_types = function
        .inputs
        .iter()
        .map(|t| types.get(&t.type_field).unwrap().clone())
        .collect::<Vec<TypeDeclaration>>();

    let name = safe_ident(&function.name);
    let fn_signature = build_fn_selector(&function.name, &fn_param_types, types)?;

    let encoded = first_four_bytes_of_sha256_hash(&fn_signature);

    let tokenized_signature = expand_selector(encoded);

    let tokenized_output = expand_fn_output(&function.output, types)?;

    let result = quote! { ContractCallHandler<#tokenized_output> };

    let (input, arg) = expand_function_arguments(function, types)?;

    let doc = expand_doc(&format!(
        "Calls the contract's `{}` (0x{}) function",
        function.name,
        hex::encode(encoded)
    ));

    let t = types
        .get(&function.output.type_field)
        .expect("couldn't find type");

    let param_type = ParamType::from_type_declaration(t, types)?;

    let tok: proc_macro2::TokenStream = format!("Some(ParamType::{})", param_type).parse().unwrap();

    let output_param = tok;

    Ok(quote! {
        #doc
        pub fn #name(&self #input) -> #result {
            Contract::method_hash(&self.wallet.get_provider().expect("Provider not set up"), self.contract_id.clone(), &self.wallet,
                #tokenized_signature, #output_param, #arg).expect("method not found (this should never happen)")
        }
    })
}

fn expand_selector(selector: Selector) -> TokenStream {
    let bytes = selector.iter().copied().map(Literal::u8_unsuffixed);
    quote! { [#( #bytes ),*] }
}

/// Expands the output of a function, i.e. what comes after `->` in a function signature.
fn expand_fn_output(
    output: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let output_type = types.get(&output.type_field).expect("couldn't find type");

    // If it's a primitive type, simply parse and expand.
    if !output_type.is_custom_type(types) {
        return expand_type(&ParamType::from_type_declaration(output_type, types)?);
    }

    // If it's a {struct, enum} as the type of a function's output, use its tokenized name only.
    match output_type.is_struct_type() {
        true => {
            let parsed_custom_type_name = extract_custom_type_name_from_abi_property(
                output_type,
                Some(CustomType::Struct),
                types,
            )?
            .parse()
            .expect("Custom type name should be a valid Rust identifier");

            Ok(parsed_custom_type_name)
        }
        false => match output_type.is_enum_type() {
            true => {
                let parsed_custom_type_name = extract_custom_type_name_from_abi_property(
                    output_type,
                    Some(CustomType::Enum),
                    types,
                )?
                .parse()
                .expect("Custom type name should be a valid Rust identifier");

                Ok(parsed_custom_type_name)
            }
            false => match output_type.has_custom_type_in_array(types) {
                true => {
                    let type_inside_array = types
                        .get(
                            &output_type
                                .components
                                .as_ref()
                                .expect("array should have components")[0]
                                .type_field,
                        )
                        .expect("couldn't find type");

                    let parsed_custom_type_name: TokenStream =
                        extract_custom_type_name_from_abi_property(
                            type_inside_array,
                            Some(
                                type_inside_array
                                    .get_custom_type()
                                    .expect("Custom type in array should be set"),
                            ),
                            types,
                        )?
                        .parse()
                        .expect("couldn't parse custom type name");

                    Ok(quote! { ::std::vec::Vec<#parsed_custom_type_name> })
                }
                false => expand_tuple_w_custom_types(output_type, types),
            },
        },
    }
}

fn expand_tuple_w_custom_types(
    output: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    if !output.has_custom_type_in_tuple(types) {
        panic!("Output is of custom type, but not an enum, struct or enum/struct inside an array/tuple. This should never happen. Output received: {:?}", output);
    }

    let mut final_signature: String = "(".into();
    let mut type_strings: Vec<String> = vec![];

    for c in output
        .components
        .as_ref()
        .expect("tuples should have components")
        .iter()
    {
        let type_string = types.get(&c.type_field).unwrap().type_field.clone();

        // If custom type is inside a tuple `(struct | enum <name>, ...)`,
        // the type signature should be only `(<name>, ...)`.
        // To do that, we remove the `STRUCT_KEYWORD` and `ENUM_KEYWORD` from it.
        let keywords_removed = remove_words(&type_string, &[STRUCT_KEYWORD, ENUM_KEYWORD]);

        let tuple_type_signature = expand_b256_into_array_form(&keywords_removed)
            .parse()
            .expect("could not parse tuple type signature");

        type_strings.push(tuple_type_signature);
    }

    final_signature.push_str(&type_strings.join(", "));
    final_signature.push(')');

    Ok(final_signature.parse().unwrap())
}

fn expand_b256_into_array_form(type_field: &str) -> String {
    let re = Regex::new(r"\bb256\b").unwrap();
    re.replace_all(type_field, "[u8; 32]").to_string()
}

fn remove_words(from: &str, words: &[&str]) -> String {
    words
        .iter()
        .fold(from.to_string(), |str_in_construction, word| {
            str_in_construction.replace(word, "")
        })
}

/// Expands the arguments in a function declaration and the same arguments as input
/// to a function call. For instance:
/// 1. The `my_arg: u32` in `pub fn my_func(my_arg: u32) -> ()`
/// 2. The `my_arg.into_token()` in `another_fn_call(my_arg.into_token())`
fn expand_function_arguments(
    fun: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<(TokenStream, TokenStream), Error> {
    let mut args = vec![];
    let mut call_args = vec![];

    for fn_type_application in &fun.inputs {
        // For each [`TypeDeclaration`] in a function input we expand:
        // 1. The name of the argument;
        // 2. The type of the argument;
        // Note that _any_ significant change in the way the JSON ABI is generated
        // could affect this function expansion.
        // TokenStream representing the name of the argument

        let name = expand_input_name(&fn_type_application.name)?;

        let param = types
            .get(&fn_type_application.type_field)
            .expect("couldn't find type");

        // TokenStream representing the type of the argument
        let kind = ParamType::from_type_declaration(param, types)?;

        // If it's a tuple, don't expand it, just use the type signature as it is (minus the string "struct " | "enum ").
        let tok = if let ParamType::Tuple(_tuple) = &kind {
            let toks = build_expanded_tuple_params(param, types)
                .expect("failed to build expanded tuple parameters");

            toks.parse::<TokenStream>().unwrap()
        } else {
            expand_input_param(
                fun,
                fn_type_application,
                &ParamType::from_type_declaration(param, types)?,
                types,
            )?
        };

        // Add the TokenStream to argument declarations
        args.push(quote! { #name: #tok });

        // This `name` TokenStream is also added to the call arguments
        if let ParamType::String(len) = &kind {
            call_args.push(quote! {Token::String(StringToken::new(#name, #len))});
        } else {
            call_args.push(name);
        }
    }

    // The final TokenStream of the argument declaration in a function declaration
    let args = quote! { #( , #args )* };

    // The final TokenStream of the arguments being passed in a function call
    // It'll look like `&[my_arg.into_token(), another_arg.into_token()]`
    // as the [`Contract`] `method_hash` function expects a slice of Tokens
    // in order to encode the call.
    let call_args = quote! { &[ #(#call_args.into_token(), )* ] };

    Ok((args, call_args))
}

// Builds a string "(type_1,type_2,type_3,...,type_n,)"
// Where each type has been expanded through `expand_type()`
// Except if it's a custom type, when just its name suffices.
// For example, a tuple coming as "(b256, struct Person)"
// Should be expanded as "([u8; 32], Person,)".
fn build_expanded_tuple_params(
    tuple_param: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<String, Error> {
    let mut toks: String = "(".to_string();
    for type_application in tuple_param
        .components
        .as_ref()
        .expect("tuple parameter should have components")
    {
        let component = types
            .get(&type_application.type_field)
            .expect("couldn't find type");

        if !component.is_custom_type(types) {
            let p = ParamType::from_type_declaration(component, types)?;
            let tok = expand_type(&p)?;
            toks.push_str(&tok.to_string());
        } else {
            let tok = component
                .type_field
                .replace(STRUCT_KEYWORD, "")
                .replace(ENUM_KEYWORD, "");
            toks.push_str(&tok.to_string());
        }
        toks.push(',');
    }
    toks.push(')');
    Ok(toks)
}

/// Expands a positional identifier string that may be empty.
///
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

// Expands the type of an argument being passed in a function declaration.
// I.e.: `pub fn my_func(my_arg: u32) -> ()`, in this case, `u32` is the
// type, coming in as a `ParamType::U32`.
fn expand_input_param(
    fun: &ABIFunction,
    type_application: &TypeApplication,
    kind: &ParamType,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    match kind {
        ParamType::Array(ty, _) => {
            let ty = expand_input_param(fun, type_application, ty, types)?;
            Ok(quote! {
                ::std::vec::Vec<#ty>
            })
        }
        ParamType::Enum(_) => {
            let t = types
                .get(&type_application.type_field)
                .expect("type not found");

            let ident = ident(&extract_custom_type_name_from_abi_property(
                t,
                Some(CustomType::Enum),
                types,
            )?);
            Ok(quote! { #ident })
        }
        ParamType::Struct(_) => {
            let t = types
                .get(&type_application.type_field)
                .expect("type not found");

            let ident = ident(&extract_custom_type_name_from_abi_property(
                t,
                Some(CustomType::Struct),
                types,
            )?);
            Ok(quote! { #ident })
        }
        // Primitive type
        _ => expand_type(kind),
    }
}

// Regarding string->TokenStream->string, refer to `custom_types_gen` tests for more details.
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
        let result = expand_function(&parsed_abi.functions[0], &all_types);

        // let result = expand_function(&the_function, &Default::default(), &Default::default());
        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `some_abi_funct` (0x00000000652399f3) function"]
            pub fn some_abi_funct(&self, s_1: MyStruct1, s_2: MyStruct2) -> ContractCallHandler<MyStruct1> {
                Contract::method_hash(
                    &self.wallet.get_provider().expect("Provider not set up"),
                    self.contract_id.clone(),
                    &self.wallet,
                    [0, 0, 0, 0, 101 , 35 , 153 , 243],
                    Some(ParamType::Struct(vec![ParamType::U64, ParamType::B256])),
                    &[s_1.into_token(), s_2.into_token(),]
                )
                .expect("method not found (this should never happen)")
            }

            "#,
        );
        let expected = expected?.to_string();

        assert_eq!(result?.to_string(), expected);
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
    #[test]
    fn test_expand_selector() {
        let result = expand_selector(Selector::default());
        assert_eq!(result.to_string(), "[0 , 0 , 0 , 0 , 0 , 0 , 0 , 0]");

        let result = expand_selector([1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(result.to_string(), "[1 , 2 , 3 , 4 , 5 , 6 , 7 , 8]");
    }

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

    #[test]
    fn will_not_replace_b256_in_middle_of_word() {
        let result = expand_b256_into_array_form("(b256, Someb256WeirdStructName, b256, b256)");

        assert_eq!(
            result,
            "([u8; 32], Someb256WeirdStructName, [u8; 32], [u8; 32])"
        );
    }
}
