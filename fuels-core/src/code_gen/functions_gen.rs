use crate::abi_encoder::ABIEncoder;
use crate::code_gen::custom_types_gen::{extract_custom_type_name_from_abi_property, CustomType};
use crate::code_gen::docs_gen::expand_doc;
use crate::errors::Error;
use crate::json_abi::{parse_param, ABIParser};
use crate::types::expand_type;
use crate::utils::{ident, safe_ident};
use crate::{ParamType, Selector};
use inflector::Inflector;
use sway_types::{Function, Property};

use proc_macro2::{Literal, TokenStream};
use quote::quote;
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
    function: &Function,
    abi_parser: &ABIParser,
    custom_enums: &HashMap<String, Property>,
    custom_structs: &HashMap<String, Property>,
) -> Result<TokenStream, Error> {
    let name = safe_ident(&function.name);
    let fn_signature = abi_parser.build_fn_selector(&function.name, &function.inputs);

    let encoded = ABIEncoder::encode_function_selector(fn_signature?.as_bytes());

    let tokenized_signature = expand_selector(encoded);
    let tokenized_output = expand_fn_outputs(&function.outputs)?;
    let result = quote! { ContractCall<#tokenized_output> };

    let (input, arg) = expand_function_arguments(function, custom_enums, custom_structs)?;

    let doc = expand_doc(&format!(
        "Calls the contract's `{}` (0x{}) function",
        function.name,
        hex::encode(encoded)
    ));

    // Here we turn `ParamType`s into a custom stringified version that's identical
    // to how we would declare a `ParamType` in Rust code. Which will then
    // be used to be tokenized and passed onto `method_hash()`.
    let mut output_params = vec![];
    for output in &function.outputs {
        let mut param_type_str: String = "ParamType::".to_owned();
        let p = parse_param(output).unwrap();
        param_type_str.push_str(&p.to_string());

        let tok: proc_macro2::TokenStream = param_type_str.parse().unwrap();

        output_params.push(tok);
    }

    let output_params_token = quote! { &[#( #output_params ),*] };

    Ok(quote! {
        #doc
        pub fn #name(&self #input) -> #result {
            Contract::method_hash(&self.fuel_client, &self.compiled,
                #tokenized_signature, #output_params_token, #arg).expect("method not found (this should never happen)")
        }
    })
}

fn expand_selector(selector: Selector) -> TokenStream {
    let bytes = selector.iter().copied().map(Literal::u8_unsuffixed);
    quote! { [#( #bytes ),*] }
}

/// Expands the output of a function, i.e. what comes after `->` in a function
/// signature.
fn expand_fn_outputs(outputs: &[Property]) -> Result<TokenStream, Error> {
    match outputs.len() {
        0 => Ok(quote! { () }),
        1 => {
            // If it's a struct as the type of a function's output, use its
            // tokenized name only. Otherwise, parse and expand.
            // The non-expansion should happen to enums as well
            if outputs[0].type_field.contains("struct ") {
                let tok: proc_macro2::TokenStream =
                    extract_custom_type_name_from_abi_property(&outputs[0], &CustomType::Struct)?
                        .parse()
                        .unwrap();
                Ok(tok)
            } else {
                expand_type(&parse_param(&outputs[0])?)
            }
        }
        _ => {
            let types = outputs
                .iter()
                .map(|param| expand_type(&parse_param(param)?))
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(quote! { (#( #types ),*) })
        }
    }
}

/// Expands the arguments in a function declaration and the same arguments as input
/// to a function call. For instance:
/// 1. The `my_arg: u32` in `pub fn my_func(my_arg: u32) -> ()`
/// 2. The `my_arg.into_token()` in `another_fn_call(my_arg.into_token())`
fn expand_function_arguments(
    fun: &Function,
    custom_enums: &HashMap<String, Property>,
    custom_structs: &HashMap<String, Property>,
) -> Result<(TokenStream, TokenStream), Error> {
    let n_inputs = fun.inputs.len();
    // Check that we have the expected types
    if n_inputs != 4 {
        return Err(Error::MissingData(format!(
            "Expected exactly 4 inputs, found {}",
            n_inputs
        )));
    }
    let given_types: Vec<String> = fun.inputs[..3]
        .iter()
        .map(|x| x.type_field.clone())
        .collect();
    let expected = ["u64", "u64", "b256"];
    if given_types != expected {
        return Err(Error::InvalidType(format!(
            "Expected the 3 first types to be {:?}, found {:?}",
            expected, given_types
        )));
    };
    // Ignore the first three arguments so we don't have to provide them when calling
    // the contracts' methods
    let first_index = 3;
    let mut args = Vec::with_capacity(1);
    let mut call_args = Vec::with_capacity(1);
    // We use a for loop because we expect exactly 4 arguments (so len(fun.inputs)==4)
    // but we might extend the number of authorized inputs in the future
    for (i, param) in fun.inputs[first_index..].iter().enumerate() {
        // For each [`Property`] in a function input we expand:
        // 1. The name of the argument;
        // 2. The type of the argument;
        // Note that _any_ significant change in the way the JSON ABI is generated
        // could affect this function expansion.
        // TokenStream representing the name of the argument
        if param.type_field == "()" {
            // This is necessary to handle methods with no user input
            continue;
        }
        let name = expand_input_name(i, &param.name);

        let opt_custom_type = match param.type_field.split_whitespace().collect::<Vec<_>>()[0] {
            "enum" => Some(&CustomType::Enum),
            "struct" => Some(&CustomType::Struct),
            _ => None,
        };

        let rust_custom_name = if let Some(c) = opt_custom_type {
            match c {
                CustomType::Enum => {
                    let name =
                        extract_custom_type_name_from_abi_property(param, opt_custom_type.unwrap())
                            .unwrap();
                    custom_enums.get(&name)
                }
                CustomType::Struct => {
                    let name =
                        extract_custom_type_name_from_abi_property(param, opt_custom_type.unwrap())
                            .unwrap();
                    custom_structs.get(&name)
                }
            }
        } else {
            None
        };

        // TokenStream representing the type of the argument
        let ty = expand_input_param(fun, &param.name, &parse_param(param)?, &rust_custom_name)?;

        // Add the TokenStream to argument declarations
        args.push(quote! { #name: #ty });

        // This `name` TokenStream is also added to the call arguments
        call_args.push(name);
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

/// Expands a positional identifier string that may be empty.
///
/// Note that this expands the parameter name with `safe_ident`, meaning that
/// identifiers that are reserved keywords get `_` appended to them.
pub fn expand_input_name(index: usize, name: &str) -> TokenStream {
    let name_str = match name {
        "" => format!("p{}", index),
        n => n.to_snake_case(),
    };
    let name = safe_ident(&name_str);

    quote! { #name }
}

// Expands the type of an argument being passed in a function declaration.
// I.e.: `pub fn my_func(my_arg: u32) -> ()`, in this case, `u32` is the
// type, coming in as a `ParamType::U32`.
fn expand_input_param(
    fun: &Function,
    param: &str,
    kind: &ParamType,
    custom_struct_name: &Option<&Property>,
) -> Result<TokenStream, Error> {
    match kind {
        ParamType::Array(ty, _) => {
            let ty = expand_input_param(fun, param, ty, custom_struct_name)?;
            Ok(quote! {
                ::std::vec::Vec<#ty>
            })
        }
        ParamType::Enum(_) => {
            let ident = ident(
                &extract_custom_type_name_from_abi_property(
                    custom_struct_name.unwrap(),
                    &CustomType::Enum,
                )?
                .to_class_case(),
            );
            Ok(quote! { #ident })
        }
        ParamType::Struct(_) => {
            let ident = ident(
                &extract_custom_type_name_from_abi_property(
                    custom_struct_name.unwrap(),
                    &CustomType::Struct,
                )?
                .to_class_case(),
            );
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
    use std::str::FromStr;
    fn generate_base_inputs() -> Vec<Property> {
        let mut inputs = Vec::with_capacity(4);
        inputs.push(Property {
            name: "gas_".to_string(),
            type_field: String::from("u64"),
            components: None,
        });
        inputs.push(Property {
            name: "amount_".to_string(),
            type_field: String::from("u64"),
            components: None,
        });
        inputs.push(Property {
            name: "color_".to_string(),
            type_field: String::from("b256"),
            components: None,
        });
        inputs
    }
    // --- expand_function ---
    #[test]
    fn test_expand_function_simple() {
        let mut the_function = Function {
            type_field: "unused".to_string(),
            inputs: generate_base_inputs(),
            name: "HelloWorld".to_string(),
            outputs: vec![],
        };
        the_function.inputs.push(Property {
            name: String::from("bimbam"),
            type_field: String::from("bool"),
            components: None,
        });
        let result = expand_function(
            &the_function,
            &ABIParser::new(),
            &Default::default(),
            &Default::default(),
        );
        let expected = TokenStream::from_str(
            r#"
#[doc = "Calls the contract's `HelloWorld` (0x000000007e387733) function"]
pub fn HelloWorld(&self, bimbam: bool) -> ContractCall<()> {
    Contract::method_hash(
        &self.fuel_client,
        &self.compiled,
        [0, 0, 0, 0, 126, 56, 119, 51],
        &[],
        &[bimbam.into_token() ,]
    )
    .expect("method not found (this should never happen)")
}
        "#,
        );
        let expected = expected.unwrap().to_string();
        assert_eq!(result.unwrap().to_string(), expected);
    }
    #[test]
    fn test_expand_function_complex() {
        let mut the_function = Function {
            type_field: "function".to_string(),
            name: "hello_world".to_string(),
            inputs: generate_base_inputs(),
            outputs: vec![
                Property {
                    name: String::from("notused"),
                    type_field: String::from("struct CoolIndieGame"),
                    components: Some(vec![
                        Property {
                            name: String::from("SuperMeat"),
                            type_field: String::from("bool"),
                            components: None,
                        },
                        Property {
                            name: String::from("BoyOrGirl"),
                            type_field: String::from("u64"),
                            components: None,
                        },
                    ]),
                },
                Property {
                    name: String::from("stillnotused"),
                    type_field: String::from("enum EntropyCirclesEnum"),
                    components: Some(vec![
                        Property {
                            name: String::from("Postcard"),
                            type_field: String::from("bool"),
                            components: None,
                        },
                        Property {
                            name: String::from("Teacup"),
                            type_field: String::from("u64"),
                            components: None,
                        },
                    ]),
                },
            ],
        };
        the_function.inputs.push(Property {
            name: String::from("the_only_allowed_input"),
            type_field: String::from("struct BurgundyBeefStruct"),
            components: Some(vec![
                Property {
                    name: String::from("Beef"),
                    type_field: String::from("bool"),
                    components: None,
                },
                Property {
                    name: String::from("BurgundyWine"),
                    type_field: String::from("u64"),
                    components: None,
                },
            ]),
        });
        let mut custom_structs = HashMap::new();
        custom_structs.insert(
            "BurgundyBeefStruct".to_string(),
            Property {
                name: "unused".to_string(),
                type_field: "struct SomeWeirdFrenchCuisine".to_string(),
                components: None,
            },
        );
        custom_structs.insert(
            "CoolIndieGame".to_string(),
            Property {
                name: "unused".to_string(),
                type_field: "struct CoolIndieGame".to_string(),
                components: None,
            },
        );
        let mut custom_enums = HashMap::new();
        custom_enums.insert(
            "EntropyCirclesEnum".to_string(),
            Property {
                name: "unused".to_string(),
                type_field: "enum EntropyCirclesEnum".to_string(),
                components: None,
            },
        );
        let abi_parser = ABIParser::new();
        let result = expand_function(&the_function, &abi_parser, &custom_enums, &custom_structs);
        // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
        let expected = TokenStream::from_str(
            r#"
#[doc = "Calls the contract's `hello_world` (0x000000001f51690a) function"]
pub fn hello_world(
    &self,
    the_only_allowed_input: SomeWeirdFrenchCuisine
) -> ContractCall<((bool , u64 ,) , (bool, u64 ,))> {
    Contract::method_hash(
        &self.fuel_client,
        &self.compiled,
        [0, 0, 0, 0, 31, 81, 105, 10],
        &[
            ParamType::Struct(vec![ParamType::Bool, ParamType::U64]),
            ParamType::Enum([Bool , U64])] , 
            &[the_only_allowed_input . into_token () ,]
    )
    .expect("method not found (this should never happen)")
}
        "#,
        );
        let expected = expected.unwrap().to_string();
        assert_eq!(result.unwrap().to_string(), expected);
    }

    // --- expand_selector ---
    #[test]
    fn test_expand_selector() {
        let result = expand_selector(Selector::default());
        assert_eq!(result.to_string(), "[0 , 0 , 0 , 0 , 0 , 0 , 0 , 0]");
        let result = expand_selector([1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(result.to_string(), "[1 , 2 , 3 , 4 , 5 , 6 , 7 , 8]");
    }

    // --- expand_fn_outputs ---
    #[test]
    fn test_expand_fn_outputs_zero_one_arg() {
        let result = expand_fn_outputs(&[]);
        assert_eq!(result.unwrap().to_string(), "()");
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: "bool".to_string(),
            components: None,
        }]);
        assert_eq!(result.unwrap().to_string(), "bool");
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: String::from("struct streaming_services"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("thistypedoesntexist"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("thistypedoesntexist"),
                    components: None,
                },
            ]),
        }]);

        // the function has inconsistent  behavior for enum compared to struct:
        // here we have to provide actual types in the components, not with the struct
        assert_eq!(result.unwrap().to_string(), "streaming_services");
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: String::from("enum unused"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("bool"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("u64"),
                    components: None,
                },
            ]),
        }]);
        assert_eq!(result.unwrap().to_string(), "(bool , u64 ,)");
    }
    #[test]
    fn test_expand_fn_outputs_no_components() {
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: String::from("struct carmaker"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("nonexistingtype"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("anotherunexistingtype"),
                    components: None,
                },
            ]),
        }]);
        // TODO: this should panic after the function is refactored
        assert_eq!(result.unwrap().to_string(), "carmaker");

        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: String::from("enum unused"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("nonexistingtype"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("anotherunexistingtype"),
                    components: None,
                },
            ]),
        }]);
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing data: cannot parse custom type with no components"
        )
    }
    #[test]
    fn test_expand_fn_outputs_two_more_components() {
        let result = expand_fn_outputs(&[
            Property {
                name: "unused".to_string(),
                type_field: String::from("bool"),
                components: None,
            },
            Property {
                name: "unused".to_string(),
                type_field: String::from("u64"),
                components: None,
            },
            Property {
                name: "unused".to_string(),
                type_field: String::from("u32"),
                components: None,
            },
        ]);
        assert_eq!(result.unwrap().to_string(), "(bool , u64 , u32)");

        let some_enum = Property {
            name: "unused".to_string(),
            type_field: String::from("enum unused"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("nonexistingtype"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("anotherunexistingtype"),
                    components: None,
                },
            ]),
        };
        let result = expand_fn_outputs(&[some_enum.clone(), some_enum]);
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing data: cannot parse custom type with no components"
        );

        let some_struct = Property {
            name: "unused".to_string(),
            type_field: String::from("struct carmaker"),
            components: Some(vec![
                Property {
                    name: String::from("unused"),
                    type_field: String::from("u64"),
                    components: None,
                },
                Property {
                    name: String::from("unused"),
                    type_field: String::from("bool"),
                    components: None,
                },
            ]),
        };
        let result = expand_fn_outputs(&[some_struct.clone(), some_struct]);
        assert_eq!(
            result.unwrap().to_string(),
            "((u64 , bool ,) , (u64 , bool ,))"
        )
    }

    // --- expand_function_argument ---
    #[test]
    fn test_expand_function_arguments() {
        let hm: HashMap<String, Property> = HashMap::new();
        let the_argument = Property {
            name: "some_argument".to_string(),
            type_field: String::from("u32"),
            components: None,
        };

        // All arguments are here
        let mut the_function = Function {
            type_field: "".to_string(),
            inputs: generate_base_inputs(),
            name: "".to_string(),
            outputs: vec![],
        };
        the_function.inputs.push(the_argument.clone());
        let result = expand_function_arguments(&the_function, &hm, &hm);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = "(, some_argument : u32,& [some_argument . into_token () ,])";
        assert_eq!(result, expected);

        // Missing the last argument
        let mut the_function = Function {
            type_field: "".to_string(),
            inputs: generate_base_inputs(),
            name: "".to_string(),
            outputs: vec![],
        };
        let result = expand_function_arguments(&the_function, &hm, &hm);
        assert!(matches!(result, Err(Error::MissingData(_))));

        // Change the `gas_` argument type
        the_function.inputs[0].type_field = String::from("bool");
        the_function.inputs.push(the_argument);
        let result = expand_function_arguments(&the_function, &hm, &hm);
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }
    #[test]
    fn test_expand_function_arguments_primitive() {
        let hm: HashMap<String, Property> = HashMap::new();
        let mut the_function = Function {
            type_field: "function".to_string(),
            inputs: generate_base_inputs(),
            name: "pip_pop".to_string(),
            outputs: vec![],
        };

        the_function.inputs.push(Property {
            name: "bim_bam".to_string(),
            type_field: String::from("u64"),
            components: None,
        });
        let result = expand_function_arguments(&the_function, &hm, &hm);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        assert_eq!(result, "(, bim_bam : u64,& [bim_bam . into_token () ,])");

        the_function.inputs[3].name = String::from("");
        let result = expand_function_arguments(&the_function, &hm, &hm);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        assert_eq!(result, "(, p0 : u64,& [p0 . into_token () ,])");
    }
    #[test]
    fn test_expand_function_arguments_composite() {
        let mut function = Function {
            type_field: "zig_zag".to_string(),
            inputs: generate_base_inputs(),
            name: "PipPopFunction".to_string(),
            outputs: vec![],
        };
        function.inputs.push(Property {
            name: "bim_bam".to_string(),
            type_field: String::from("struct CarMaker"),
            // Not parsed, so can be empty but not None
            components: Some(vec![]),
        });
        let mut custom_structs = HashMap::new();
        custom_structs.insert(
            "CarMaker".to_string(),
            Property {
                name: "unused".to_string(),
                type_field: "struct CarMaker".to_string(),
                components: None,
            },
        );

        let result = expand_function_arguments(&function, &custom_structs, &custom_structs);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = r#"(, bim_bam : CarMaker,& [bim_bam . into_token () ,])"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_expand_function_argument_empty_fourth() {
        let mut function = Function {
            type_field: "zig_zag".to_string(),
            inputs: generate_base_inputs(),
            name: "PipPopFunction".to_string(),
            outputs: vec![],
        };
        function.inputs.push(Property {
            name: "".to_string(),
            type_field: "()".to_string(),
            components: None,
        });
        let custom_structs = HashMap::new();
        let result = expand_function_arguments(&function, &custom_structs, &custom_structs);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = r#"(,& [])"#;
        assert_eq!(result, expected);
    }

    // --- expand_input_name ---
    #[test]
    fn test_expand_input_name() {
        let result = expand_input_name(0, "CamelCaseHello");
        assert_eq!(result.to_string(), "camel_case_hello");
        let result = expand_input_name(1080, "");
        assert_eq!(result.to_string(), "p1080");
        let result = expand_input_name(0, "if");
        assert_eq!(result.to_string(), "if_");
        let result = expand_input_name(0, "let");
        assert_eq!(result.to_string(), "let_");
    }

    // --- expand_input_param ---
    #[test]
    fn test_expand_input_param_primitive() {
        let def = Function::default();
        let result = expand_input_param(&def, "unused", &ParamType::Bool, &None);
        assert_eq!(result.unwrap().to_string(), "bool");
        let result = expand_input_param(&def, "unused", &ParamType::U64, &None);
        assert_eq!(result.unwrap().to_string(), "u64");
        let result = expand_input_param(&def, "unused", &ParamType::String(10), &None);
        assert_eq!(result.unwrap().to_string(), "String");
    }
    #[test]
    fn test_expand_input_param_array() {
        let array_type = ParamType::Array(Box::new(ParamType::U64), 10);
        let result = expand_input_param(&Function::default(), "unused", &array_type, &None);
        assert_eq!(result.unwrap().to_string(), ":: std :: vec :: Vec < u64 >");
    }
    #[test]
    fn test_expand_input_param_struct_name() {
        let def = Function::default();
        let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
        let struct_prop = Property {
            name: String::from("unused"),
            type_field: String::from("struct babies"),
            components: None,
        };
        let struct_name = Some(&struct_prop);
        let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
        // Notice the removed plural!
        assert_eq!(result.unwrap().to_string(), "Baby");
    }
    #[test]
    fn test_expand_input_param_struct_wrong_name() {
        let def = Function::default();
        let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
        let struct_prop = Property {
            name: String::from("unused"),
            type_field: String::from("not_the_right_format"),
            components: None,
        };
        let struct_name = Some(&struct_prop);
        let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
        assert!(matches!(result, Err(Error::MissingData(_))));
    }
    #[test]
    fn test_expand_input_param_struct_with_enum_name() {
        let def = Function::default();
        let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
        let struct_prop = Property {
            name: String::from("unused"),
            type_field: String::from("enum butitsastruct"),
            components: None,
        };
        let struct_name = Some(&struct_prop);
        let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }
}
