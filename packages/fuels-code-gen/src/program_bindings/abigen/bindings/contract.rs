use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, TokenStreamExt};

use crate::{
    error::Result,
    program_bindings::{
        abigen::{
            bindings::function_generator::FunctionGenerator,
            configurables::generate_code_for_configurable_constants,
            logs::log_formatters_instantiation_code,
        },
        generated_code::GeneratedCode,
    },
    utils::{ident, TypePath},
};

use fuel_abi_types::abi::full_program::{FullABIFunction, FullProgramABI};

pub(crate) fn contract_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
) -> Result<GeneratedCode> {
    if no_std {
        return Ok(GeneratedCode::default());
    }

    let log_formatters =
        log_formatters_instantiation_code(quote! {contract_id.clone().into()}, &abi.logged_types);

    let methods_name = ident(&format!("{name}Methods"));

    let contract_functions = expand_functions(&abi.functions)?;

    let configuration_struct_name = ident(&format!("{name}Configurables"));
    let constant_configuration_code =
        generate_code_for_configurable_constants(&configuration_struct_name, &abi.configurables)?;

    let code = quote! {
        pub struct #name<T: ::fuels::accounts::Account> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder
        }

        impl<T: ::fuels::accounts::Account> #name<T>
        {
            pub fn new(
                contract_id: impl ::core::convert::Into<::fuels::types::bech32::Bech32ContractId>,
                account: T,
            ) -> Self {
                let contract_id: ::fuels::types::bech32::Bech32ContractId = contract_id.into();
                let log_decoder = ::fuels::programs::logs::LogDecoder { log_formatters: #log_formatters };
                Self { contract_id, account, log_decoder }
            }

            pub fn contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                &self.contract_id
            }

            pub fn account(&self) -> T {
                self.account.clone()
            }

            pub fn with_account<U: ::fuels::accounts::Account>(&self, mut account: U) -> ::fuels::types::errors::Result<#name<U>> {
                ::core::result::Result::Ok(#name { contract_id: self.contract_id.clone(), account, log_decoder: self.log_decoder.clone()})
            }

            pub async fn get_balances(&self) -> ::fuels::types::errors::Result<::std::collections::HashMap<::std::string::String, u64>> {
                ::fuels::accounts::ViewOnlyAccount::try_provider(&self.account)?
                                  .get_contract_balances(&self.contract_id)
                                  .await
                                  .map_err(::std::convert::Into::into)
            }

            pub fn methods(&self) -> #methods_name<T> {
                #methods_name {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone()
                }
            }
        }

        // Implement struct that holds the contract methods
        pub struct #methods_name<T: ::fuels::accounts::Account> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder
        }

        impl<T: ::fuels::accounts::Account> #methods_name<T> {
            #contract_functions
        }

        impl<T: ::fuels::accounts::Account>
            ::fuels::programs::contract::SettableContract for #name<T>
        {
            fn id(&self) -> ::fuels::types::bech32::Bech32ContractId {
                self.contract_id.clone()
            }

            fn log_decoder(&self) -> ::fuels::programs::logs::LogDecoder {
                self.log_decoder.clone()
            }
        }

        #constant_configuration_code
    };

    // All publicly available types generated above should be listed here.
    let type_paths = [name, &methods_name, &configuration_struct_name]
        .map(|type_name| TypePath::new(type_name).expect("We know the given types are not empty"))
        .into_iter()
        .collect();

    Ok(GeneratedCode::new(code, type_paths, no_std))
}

fn expand_functions(functions: &[FullABIFunction]) -> Result<TokenStream> {
    functions
        .iter()
        .map(expand_fn)
        .fold_ok(TokenStream::default(), |mut all_code, code| {
            all_code.append_all(code);
            all_code
        })
}

/// Transforms a function defined in [`FullABIFunction`] into a [`TokenStream`]
/// that represents that same function signature as a Rust-native function
/// declaration.
///
/// The generated function prepares the necessary data and proceeds to call
/// [::fuels_contract::contract::method_hash] for the actual call.
pub(crate) fn expand_fn(abi_fun: &FullABIFunction) -> Result<TokenStream> {
    let mut generator = FunctionGenerator::new(abi_fun)?;

    generator.set_doc(format!(
        "Calls the contract's `{}` function",
        abi_fun.name(),
    ));

    let original_output = generator.output_type();
    generator.set_output_type(
        quote! {::fuels::programs::contract::ContractCallHandler<T, #original_output> },
    );

    let fn_selector = generator.fn_selector();
    let arg_tokens = generator.tokenized_args();
    let is_payable = abi_fun.is_payable();
    let body = quote! {
            ::fuels::programs::contract::method_hash(
                self.contract_id.clone(),
                self.account.clone(),
                #fn_selector,
                &#arg_tokens,
                self.log_decoder.clone(),
                #is_payable,
            )
            .expect("method not found (this should never happen)")
    };
    generator.set_body(body);

    Ok(generator.into())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::abi::program::{ABIFunction, ProgramABI, TypeApplication, TypeDeclaration};

    use super::*;

    #[test]
    fn test_expand_fn_simple_abi() -> Result<()> {
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
        let result = expand_fn(&FullABIFunction::from_counterpart(
            &parsed_abi.functions[0],
            &types,
        )?)?;

        let expected = quote! {
            #[doc = "Calls the contract's `some_abi_funct` function"]
            pub fn some_abi_funct(
                &self,
                s_1: self::MyStruct1,
                s_2: self::MyStruct2
            ) -> ::fuels::programs::contract::ContractCallHandler<T, self::MyStruct1> {
                ::fuels::programs::contract::method_hash(
                    self.contract_id.clone(),
                    self.account.clone(),
                    ::fuels::core::codec::resolve_fn_selector(
                        "some_abi_funct",
                        &[
                            <self::MyStruct1 as ::fuels::core::traits::Parameterize>::param_type(),
                            <self::MyStruct2 as ::fuels::core::traits::Parameterize>::param_type()
                        ]
                    ),
                    &[
                        ::fuels::core::traits::Tokenizable::into_token(s_1),
                        ::fuels::core::traits::Tokenizable::into_token(s_2)
                    ],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
        };

        assert_eq!(result.to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_expand_fn_simple() -> Result<()> {
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
        let result = expand_fn(&FullABIFunction::from_counterpart(&the_function, &types)?);

        let expected = quote! {
            #[doc = "Calls the contract's `HelloWorld` function"]
            pub fn HelloWorld(&self, bimbam: bool) -> ::fuels::programs::contract::ContractCallHandler<T, ()> {
                ::fuels::programs::contract::method_hash(
                    self.contract_id.clone(),
                    self.account.clone(),
                    ::fuels::core::codec::resolve_fn_selector(
                        "HelloWorld",
                        &[<bool as ::fuels::core::traits::Parameterize>::param_type()]
                    ),
                    &[::fuels::core::traits::Tokenizable::into_token(bimbam)],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
        };

        assert_eq!(result?.to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn test_expand_fn_complex() -> Result<()> {
        // given
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

        // when
        let result = expand_fn(&FullABIFunction::from_counterpart(&the_function, &types)?);

        //then

        // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
        let expected = quote! {
            #[doc = "Calls the contract's `hello_world` function"]
            pub fn hello_world(
                &self,
                the_only_allowed_input: self::SomeWeirdFrenchCuisine
            ) -> ::fuels::programs::contract::ContractCallHandler<T, self::EntropyCirclesEnum> {
                ::fuels::programs::contract::method_hash(
                    self.contract_id.clone(),
                    self.account.clone(),
                    ::fuels::core::codec::resolve_fn_selector(
                        "hello_world",
                        &[<self::SomeWeirdFrenchCuisine as ::fuels::core::traits::Parameterize>::param_type()]
                    ),
                    &[::fuels::core::traits::Tokenizable::into_token(
                        the_only_allowed_input
                    )],
                    self.log_decoder.clone(),
                    false,
                )
                .expect("method not found (this should never happen)")
            }
        };

        assert_eq!(result?.to_string(), expected.to_string());

        Ok(())
    }
}
