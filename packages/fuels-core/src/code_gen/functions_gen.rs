use crate::abi_encoder::ABIEncoder;
use crate::code_gen::custom_types_gen::extract_custom_type_name_from_abi_property;
use crate::code_gen::docs_gen::expand_doc;
use crate::errors::Error;
use crate::json_abi::{parse_param, ABIParser};
use crate::types::expand_type;
use crate::utils::{ident, safe_ident};
use crate::{ParamType, Selector};
use fuels_types::{CustomType, Function, Property, ENUM_KEYWORD, STRUCT_KEYWORD};
use inflector::Inflector;
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
    let result = quote! { ContractCallHandler<#tokenized_output> };

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
            Contract::method_hash(&self.wallet.get_provider().expect("Provider not set up"), self.contract_id, &self.wallet,
                #tokenized_signature, #output_params_token, #arg).expect("method not found (this should never happen)")
        }
    })
}

fn expand_selector(selector: Selector) -> TokenStream {
    let bytes = selector.iter().copied().map(Literal::u8_unsuffixed);
    quote! { [#( #bytes ),*] }
}

/// Expands the output of a function, i.e. what comes after `->` in a function signature.
fn expand_fn_outputs(outputs: &[Property]) -> Result<TokenStream, Error> {
    match outputs.len() {
        0 => Ok(quote! { () }),
        1 => {
            let output = outputs.first().expect("Outputs shouldn't not be empty");

            // If it's a primitive type, simply parse and expand.
            if !output.is_custom_type() {
                return expand_type(&parse_param(output)?);
            }

            // If it's a {struct, enum} as the type of a function's output, use its tokenized name only.
            match output.is_struct_type() {
                true => {
                    let parsed_custom_type_name = extract_custom_type_name_from_abi_property(
                        output,
                        Some(CustomType::Struct),
                    )?
                    .parse()
                    .expect("Custom type name should be a valid Rust identifier");

                    Ok(parsed_custom_type_name)
                }
                false => match output.is_enum_type() {
                    true => {
                        let parsed_custom_type_name = extract_custom_type_name_from_abi_property(
                            output,
                            Some(CustomType::Enum),
                        )?
                        .parse()
                        .expect("Custom type name should be a valid Rust identifier");

                        Ok(parsed_custom_type_name)
                    }
                    false => match output.has_custom_type_in_array() {
                        true => {
                            let parsed_custom_type_name: TokenStream =
                                extract_custom_type_name_from_abi_property(
                                    output,
                                    Some(
                                        output
                                            .get_custom_type()
                                            .expect("Custom type in array should be set"),
                                    ),
                                )?
                                .parse()
                                .unwrap();

                            Ok(quote! { ::std::vec::Vec<#parsed_custom_type_name> })
                        }
                        false => match output.has_custom_type_in_tuple() {
                            // If custom type is inside a tuple `(struct | enum <name>, ...)`,
                            // the type signature should be only `(<name>, ...)`.
                            // To do that, we remove the `STRUCT_KEYWORD` and `ENUM_KEYWORD` from it.
                            true => {
                                let tuple_type_signature: TokenStream = output
                                    .type_field
                                    .replace(STRUCT_KEYWORD, "")
                                    .replace(ENUM_KEYWORD, "")
                                    .parse()
                                    .expect("could not parse tuple type signature");

                                Ok(tuple_type_signature)
                            }
                            false => {
                                panic!("{}", format!("Output is of custom type, but not an enum, struct or enum/struct inside an array/tuple. This shouldn't never happen. Output received: {:?}", output));
                            }
                        },
                    },
                },
            }
        }
        // Recursively expand the outputs
        _ => {
            let types = outputs
                .iter()
                .map(|param| expand_fn_outputs(&[param.clone()]))
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
    let mut args = vec![];
    let mut call_args = vec![];

    for (i, param) in fun.inputs.iter().enumerate() {
        // For each [`Property`] in a function input we expand:
        // 1. The name of the argument;
        // 2. The type of the argument;
        // Note that _any_ significant change in the way the JSON ABI is generated
        // could affect this function expansion.
        // TokenStream representing the name of the argument
        let name = expand_input_name(i, &param.name);

        let custom_property = match param.is_custom_type() {
            false => None,
            true => {
                if param.is_enum_type() {
                    let name =
                        extract_custom_type_name_from_abi_property(param, Some(CustomType::Enum))
                            .expect("couldn't extract enum name from ABI property");
                    custom_enums.get(&name)
                } else if param.is_struct_type() {
                    let name =
                        extract_custom_type_name_from_abi_property(param, Some(CustomType::Struct))
                            .expect("couldn't extract struct name from ABI property");
                    custom_structs.get(&name)
                } else {
                    match param.has_custom_type_in_array() {
                        true => match param.get_custom_type() {
                            Some(custom_type) => {
                                let name = extract_custom_type_name_from_abi_property(
                                    param,
                                    Some(custom_type),
                                )
                                .expect("couldn't extract custom type name from ABI property");

                                match custom_type {
                                    CustomType::Enum => custom_enums.get(&name),
                                    CustomType::Struct => custom_structs.get(&name),
                                }
                            }
                            None => {
                                return Err(Error::InvalidType(format!(
                                    "Custom type in array is not a struct or enum. Type: {:?}",
                                    param
                                )))
                            }
                        },
                        false => None,
                    }
                }
            }
        };

        // TokenStream representing the type of the argument
        let kind = parse_param(param)?;

        // If it's a tuple, don't expand it, just use the type signature as it is (minus the string "struct " | "enum ").
        let tok = if let ParamType::Tuple(_tuple) = kind {
            let toks = build_expanded_tuple_params(param)
                .expect("failed to build expanded tuple parameters");

            toks.parse::<TokenStream>().unwrap()
        } else {
            expand_input_param(fun, &param.name, &parse_param(param)?, &custom_property)?
        };

        // Add the TokenStream to argument declarations
        args.push(quote! { #name: #tok });

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

// Builds a string "(type_1,type_2,type_3,...,type_n,)"
// Where each type has been expanded through `expand_type()`
// Except if it's a custom type, when just its name suffices.
// For example, a tuple coming as "(b256, struct Person)"
// Should be expanded as "([u8; 32], Person,)".
fn build_expanded_tuple_params(tuple_param: &Property) -> Result<String, Error> {
    let mut toks: String = "(".to_string();
    for component in tuple_param
        .components
        .as_ref()
        .expect("tuple parameter should have components")
    {
        if !component.is_custom_type() {
            let p = parse_param(component)?;
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
    custom_type_property: &Option<&Property>,
) -> Result<TokenStream, Error> {
    match kind {
        ParamType::Array(ty, _) => {
            let ty = expand_input_param(fun, param, ty, custom_type_property)?;
            Ok(quote! {
                ::std::vec::Vec<#ty>
            })
        }
        ParamType::Enum(_) => {
            let ident = ident(
                &extract_custom_type_name_from_abi_property(
                    custom_type_property.expect("Custom type property not found for enum"),
                    Some(CustomType::Enum),
                )?
                .to_class_case(),
            );
            Ok(quote! { #ident })
        }
        ParamType::Struct(_) => {
            let ident = ident(
                &extract_custom_type_name_from_abi_property(
                    custom_type_property.expect("Custom type property not found for struct"),
                    Some(CustomType::Struct),
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
    use crate::EnumVariants;
    use std::str::FromStr;

    #[test]
    fn test_expand_function_simple() {
        let mut the_function = Function {
            type_field: "unused".to_string(),
            inputs: vec![],
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
#[doc = "Calls the contract's `HelloWorld` (0x0000000097d4de45) function"]
pub fn HelloWorld(&self, bimbam: bool) -> ContractCallHandler<()> {
    Contract::method_hash(
        &self.wallet.get_provider().expect("Provider not set up"),
        self.contract_id,
        &self.wallet,
        [0, 0, 0, 0, 151, 212, 222, 69],
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
            inputs: vec![],
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
#[doc = "Calls the contract's `hello_world` (0x0000000076b25a24) function"]
pub fn hello_world(
    &self,
    the_only_allowed_input: SomeWeirdFrenchCuisine
) -> ContractCallHandler<(CoolIndieGame , EntropyCirclesEnum)> {
    Contract::method_hash(
        &self.wallet.get_provider().expect("Provider not set up"),
        self.contract_id,
        &self.wallet,
        [0, 0, 0, 0, 118, 178, 90, 36],
        &[
            ParamType::Struct(vec![ParamType::Bool, ParamType::U64]),
            ParamType::Enum(EnumVariants::new(vec![ParamType::Bool, ParamType::U64]).unwrap())],
        &[the_only_allowed_input.into_token() ,]
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
    fn test_expand_fn_outputs() {
        let result = expand_fn_outputs(&[]);
        assert_eq!(result.unwrap().to_string(), "()");

        // Primitive type
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: "bool".to_string(),
            components: None,
        }]);
        assert_eq!(result.unwrap().to_string(), "bool");

        // Struct type
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
        assert_eq!(result.unwrap().to_string(), "streaming_services");

        // Enum type
        let result = expand_fn_outputs(&[Property {
            name: "unused".to_string(),
            type_field: String::from("enum StreamingServices"),
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
        assert_eq!(result.unwrap().to_string(), "StreamingServices");
    }
    #[test]
    fn test_expand_fn_outputs_two_more_arguments() {
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

        let two_empty_components = vec![
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
        ];

        let some_enum = Property {
            name: "unused".to_string(),
            type_field: String::from("enum Carmaker"),
            components: Some(two_empty_components.clone()),
        };
        let result = expand_fn_outputs(&[some_enum.clone(), some_enum]);
        assert_eq!(result.unwrap().to_string(), "(Carmaker , Carmaker)");

        let some_struct = Property {
            name: "unused".to_string(),
            type_field: String::from("struct Carmaker"),
            components: Some(two_empty_components),
        };
        let result = expand_fn_outputs(&[some_struct.clone(), some_struct]);
        assert_eq!(result.unwrap().to_string(), "(Carmaker , Carmaker)")
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
            inputs: vec![],
            name: "".to_string(),
            outputs: vec![],
        };
        the_function.inputs.push(the_argument);

        let result = expand_function_arguments(&the_function, &hm, &hm);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = "(, some_argument : u32,& [some_argument . into_token () ,])";
        assert_eq!(result, expected);
    }
    #[test]
    fn test_expand_function_arguments_primitive() {
        let hm: HashMap<String, Property> = HashMap::new();
        let mut the_function = Function {
            type_field: "function".to_string(),
            inputs: vec![],
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

        the_function.inputs[0].name = String::from("");
        let result = expand_function_arguments(&the_function, &hm, &hm);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        assert_eq!(result, "(, p0 : u64,& [p0 . into_token () ,])");
    }
    #[test]
    fn test_expand_function_arguments_composite() {
        let mut function = Function {
            type_field: "zig_zag".to_string(),
            inputs: vec![],
            name: "PipPopFunction".to_string(),
            outputs: vec![],
        };
        function.inputs.push(Property {
            name: "bim_bam".to_string(),
            type_field: String::from("struct CarMaker"),
            components: Some(vec![Property {
                name: "name".to_string(),
                type_field: "str[5]".to_string(),
                components: None,
            }]),
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
        let mut custom_enums = HashMap::new();
        custom_enums.insert(
            "Cocktail".to_string(),
            Property {
                name: "Cocktail".to_string(),
                type_field: "enum Cocktail".to_string(),
                components: Some(vec![Property {
                    name: "variant".to_string(),
                    type_field: "u32".to_string(),
                    components: None,
                }]),
            },
        );

        let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = r#"(, bim_bam : CarMaker,& [bim_bam . into_token () ,])"#;
        assert_eq!(result, expected);

        function.inputs[0].type_field = "enum Cocktail".to_string();
        let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
        let (args, call_args) = result.unwrap();
        let result = format!("({},{})", args, call_args);
        let expected = r#"(, bim_bam : Cocktail,& [bim_bam . into_token () ,])"#;
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
    fn test_expand_input_param_custom_type() {
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

        let enum_type =
            ParamType::Enum(EnumVariants::new(vec![ParamType::U8, ParamType::U32]).unwrap());
        let enum_prop = Property {
            name: String::from("unused"),
            type_field: String::from("enum babies"),
            components: None,
        };
        let enum_name = Some(&enum_prop);
        let result = expand_input_param(&def, "unused", &enum_type, &enum_name);
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
