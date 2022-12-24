use crate::code_gen::full_abi_types::{FullABIFunction, FullTypeApplication, FullTypeDeclaration};
use crate::code_gen::utils::{param_type_calls, Component};
use crate::code_gen::{docs_gen::expand_doc, resolved_type, resolved_type::ResolvedType};
use crate::utils::safe_ident;
use fuels_types::errors::Error;
use proc_macro2::TokenStream;
use quote::quote;
use resolved_type::resolve_type;
use std::collections::HashSet;

/// Functions used by the Abigen to expand functions defined in an ABI spec.

/// Transforms a function defined in [`ABIFunction`] into a [`TokenStream`]
/// that represents that same function signature as a Rust-native function
/// declaration.
///
/// The actual logic inside the function is the function `method_hash` under
/// [`Contract`], which is responsible for encoding
/// the function selector and the function parameters that will be used
/// in the actual contract call.
///
/// [`Contract`]: fuels_contract::contract::Contract
// TODO (oleksii/docs): linkify the above `Contract` link properly
pub(crate) fn expand_function(
    abi_fun: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream, Error> {
    let mut generator = FunctionGenerator::new(abi_fun, shared_types)?;

    generator.set_doc(format!(
        "Calls the contract's `{}` function",
        abi_fun.name(),
    ));

    let original_output = generator.output_type();
    generator.set_output_type(
        quote! {::fuels::contract::contract::ContractCallHandler<#original_output> },
    );

    let fn_selector = generator.fn_selector();
    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let provider = self.wallet.get_provider().expect("Provider not set up");
            let log_decoder = ::fuels::contract::logs::LogDecoder{logs_map: self.logs_map.clone()};
            ::fuels::contract::contract::Contract::method_hash(
                &provider,
                self.contract_id.clone(),
                &self.wallet,
                #fn_selector,
                &#arg_tokens,
                log_decoder
            )
            .expect("method not found (this should never happen)")
    };
    generator.set_body(body);

    Ok(generator.into())
}

#[derive(Debug)]
struct FunctionGenerator {
    name: String,
    args: Vec<Component>,
    output_type: TokenStream,
    body: TokenStream,
    doc: Option<String>,
}

impl FunctionGenerator {
    pub fn new(
        fun: &FullABIFunction,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<Self, Error> {
        let args = function_arguments(fun.inputs(), shared_types)?;

        let output_type = resolve_fn_output_type(fun, shared_types)?;

        Ok(Self {
            name: fun.name().to_string(),
            args,
            output_type: output_type.into(),
            body: Default::default(),
            doc: None,
        })
    }

    pub fn set_body(&mut self, body: TokenStream) -> &mut Self {
        self.body = body;
        self
    }

    pub fn set_doc(&mut self, text: String) -> &mut Self {
        self.doc = Some(text);
        self
    }

    pub fn fn_selector(&self) -> TokenStream {
        let param_type_calls = param_type_calls(&self.args);

        let name = &self.name;
        quote! {::fuels::core::code_gen::function_selector::resolve_fn_selector(#name, &[#(#param_type_calls),*])}
    }

    pub fn tokenized_args(&self) -> TokenStream {
        let arg_names = self.args.iter().map(|component| &component.field_name);
        quote! {[#(::fuels::core::Tokenizable::into_token(#arg_names)),*]}
    }

    pub fn set_output_type(&mut self, output_type: TokenStream) -> &mut Self {
        self.output_type = output_type;
        self
    }

    pub fn output_type(&self) -> &TokenStream {
        &self.output_type
    }
}

impl From<&FunctionGenerator> for TokenStream {
    fn from(fun: &FunctionGenerator) -> Self {
        let name = safe_ident(&fun.name);
        let doc = fun
            .doc
            .as_ref()
            .map(|text| expand_doc(text))
            .unwrap_or_default();

        let arg_declarations = fun.args.iter().map(|component| {
            let name = &component.field_name;
            let field_type: TokenStream = (&component.field_type).into();
            quote! { #name: #field_type }
        });

        let output_type = fun.output_type();
        let body = &fun.body;

        quote! {
            #doc
            pub fn #name(&self #(,#arg_declarations)*) -> #output_type {
                #body
            }
        }
    }
}

impl From<FunctionGenerator> for TokenStream {
    fn from(fun: FunctionGenerator) -> Self {
        (&fun).into()
    }
}

/// Generate the `main` function of a script
pub(crate) fn generate_script_main_function(
    fun: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<TokenStream, Error> {
    let mut generator = FunctionGenerator::new(fun, shared_types)?;

    let original_output_type = generator.output_type();
    generator
        .set_output_type(
            quote! {::fuels::contract::script_calls::ScriptCallHandler<#original_output_type> },
        )
        .set_doc("Run the script's `main` function with the provided arguments".to_string());

    let arg_tokens = generator.tokenized_args();
    let body = quote! {
            let script_binary = ::std::fs::read(&self.binary_filepath)
                                        .expect("Could not read from binary filepath");
            let encoded_args = ::fuels::core::abi_encoder::ABIEncoder::encode(&#arg_tokens).expect("Cannot encode script arguments");
            let provider = self.wallet.get_provider().expect("Provider not set up").clone();
            let log_decoder = ::fuels::contract::logs::LogDecoder{logs_map: self.logs_map.clone()};

            ::fuels::contract::script_calls::ScriptCallHandler::new(
                script_binary,
                encoded_args,
                self.wallet.clone(),
                provider,
                log_decoder
            )
    };

    generator.set_body(body);

    Ok(generator.into())
}

fn resolve_fn_output_type(
    function: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let output_type = resolve_type(function.output(), shared_types)?;
    if output_type.uses_vectors() {
        Err(Error::CompilationError(format!(
            "function '{}' contains a vector in its return type. This currently isn't supported.",
            function.name()
        )))
    } else {
        Ok(output_type)
    }
}

fn function_arguments(
    inputs: &[FullTypeApplication],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<Vec<Component>, Error> {
    inputs
        .iter()
        .map(|input| Component::new(input, true, shared_types))
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|e| Error::InvalidType(e.to_string()))
}

// Regarding string->TokenStream->string, refer to `custom_types` tests for more details.
#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::{ABIFunction, ProgramABI, TypeApplication, TypeDeclaration};
    use std::collections::HashMap;
    use std::str::FromStr;

    #[test]
    fn test_expand_function_simple_abi() -> Result<(), Error> {
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
        let types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        // Grabbing the one and only function in it.
        let result = expand_function(
            &FullABIFunction::from_counterpart(&parsed_abi.functions[0], &types)?,
            &HashSet::default(),
        )?;

        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `some_abi_funct` function"]
            pub fn some_abi_funct(
                &self,
                s_1: self::MyStruct1,
                s_2: self::MyStruct2
            ) -> ::fuels::contract::contract::ContractCallHandler<self::MyStruct1> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let encoded_fn_selector = ::fuels::core::code_gen::function_selector::resolve_fn_selector(
                    "some_abi_funct",
                    &[
                        <self::MyStruct1 as ::fuels::core::Parameterize> ::param_type(),
                        <self::MyStruct2 as ::fuels::core::Parameterize> ::param_type()
                    ]
                );
                let tokens = [
                    ::fuels::core::Tokenizable::into_token(s_1),
                    ::fuels::core::Tokenizable::into_token(s_2)
                ];
                let log_decoder = ::fuels::contract::logs::LogDecoder {
                    logs_map: self.logs_map.clone()
                };
                ::fuels::contract::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    encoded_fn_selector,
                    &tokens,
                    log_decoder
                )
                .expect("method not found (this should never happen)")
            }
            "#,
        )?.to_string();

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
        let result = expand_function(
            &FullABIFunction::from_counterpart(&the_function, &types)?,
            &HashSet::default(),
        );

        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `HelloWorld` function"]
            pub fn HelloWorld(&self, bimbam: bool) -> ::fuels::contract::contract::ContractCallHandler<()> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let encoded_fn_selector = ::fuels::core::code_gen::function_selector::resolve_fn_selector(
                    "HelloWorld",
                    &[<bool as ::fuels::core::Parameterize> ::param_type()]
                );
                let tokens = [::fuels::core::Tokenizable::into_token(bimbam)];
                let log_decoder = ::fuels::contract::logs::LogDecoder {
                    logs_map: self.logs_map.clone()
                };
                ::fuels::contract::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    encoded_fn_selector,
                    &tokens,
                    log_decoder
                )
                .expect("method not found (this should never happen)")
            }
            "#,
        )?.to_string();

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
            ..Default::default()
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
        let result = expand_function(
            &FullABIFunction::from_counterpart(&the_function, &types)?,
            &HashSet::default(),
        );
        // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `hello_world` function"]
            pub fn hello_world(
                &self,
                the_only_allowed_input: self::SomeWeirdFrenchCuisine
            ) -> ::fuels::contract::contract::ContractCallHandler<self::EntropyCirclesEnum> {
                let provider = self.wallet.get_provider().expect("Provider not set up");
                let encoded_fn_selector = ::fuels::core::code_gen::function_selector::resolve_fn_selector(
                    "hello_world",
                    &[<self::SomeWeirdFrenchCuisine as ::fuels::core::Parameterize> ::param_type()]
                );
                let tokens = [::fuels::core::Tokenizable::into_token(
                    the_only_allowed_input
                )];
                let log_decoder = ::fuels::contract::logs::LogDecoder {
                    logs_map: self.logs_map.clone()
                };
                ::fuels::contract::contract::Contract::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.wallet,
                    encoded_fn_selector,
                    &tokens,
                    log_decoder
                )
                .expect("method not found (this should never happen)")
            }
            "#,
        )?.to_string();

        assert_eq!(result?.to_string(), expected);

        Ok(())
    }

    // --- expand_function_argument ---
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
        let result = function_arguments(
            FullABIFunction::from_counterpart(&the_function, &types)?.inputs(),
            &HashSet::default(),
        )?;
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
        let result = function_arguments(
            FullABIFunction::from_counterpart(&the_function, &types)?.inputs(),
            &HashSet::default(),
        )?;
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
        let result = function_arguments(
            FullABIFunction::from_counterpart(&function, &types)?.inputs(),
            &HashSet::default(),
        )?;

        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "self :: CarMaker");

        function.inputs[0].type_id = 2;
        let result = function_arguments(
            FullABIFunction::from_counterpart(&function, &types)?.inputs(),
            &HashSet::default(),
        )?;

        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "self :: Cocktail");

        Ok(())
    }
}
