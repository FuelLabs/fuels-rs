use fuel_core::service::Config as CoreConfig;
use fuel_core::service::FuelService;
use fuel_gql_client::fuel_tx::{AssetId, ContractId, Receipt};
use fuels::contract::contract::MultiContractCallHandler;
use fuels::contract::predicate::Predicate;
use fuels::prelude::{
    abigen, launch_custom_provider_and_get_wallets, launch_provider_and_get_wallet,
    setup_contract_test, setup_multiple_assets_coins, setup_single_asset_coins,
    setup_test_provider, CallParameters, Config, Contract, Error, Provider, Salt, TxParameters,
    WalletUnlocked, WalletsConfig, DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS,
};
use fuels_core::abi_encoder::ABIEncoder;
use fuels_core::parameters::StorageConfiguration;
use fuels_core::tx::{Address, Bytes32, StorageSlot};
use fuels_core::types::{Bits256, Byte};
use fuels_core::Tokenizable;
use fuels_core::{constants::BASE_ASSET_ID, Token};
use fuels_signers::fuel_crypto::SecretKey;
use fuels_test_helpers::setup_single_message;
use fuels_types::bech32::Bech32Address;
use sha2::{Digest, Sha256};
use std::iter;
use std::str::FromStr;

use fuel_core_interfaces::model::Message;
use fuel_gql_client::client::schema::message::Message as OtherMessage;

/// Note: all the tests and examples below require pre-compiled Sway projects.
/// To compile these projects, run `cargo run --bin build-test-projects`.
/// It will build all test projects, creating their respective binaries,
/// ABI files, and lock files. These are not to be committed to the repository.

/// #[ctor::ctor] Marks a function or static variable as a library/executable constructor.
/// This uses OS-specific linker sections to call a specific function at load time.
#[cfg(test)]
#[ctor::ctor]
fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

fn null_contract_id() -> String {
    // a bech32 contract address that decodes to ~[0u8;32]
    String::from("fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2")
}

#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        "packages/fuels/tests/takes_ints_returns_bool-abi.json",
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_ints_returns_bool(42);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("000000009593586c000000000000002a", encoded);
}

#[tokio::test]
async fn compile_bindings_from_inline_contract() -> Result<(), Error> {
    // ANCHOR: bindings_from_inline_contracts
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
                {
                    "typeId": 0,
                    "type": "bool",
                    "components": null,
                    "typeParameters": null
                },
                {
                    "typeId": 1,
                    "type": "u32",
                    "components": null,
                    "typeParameters": null
                }
            ],
            "functions": [
                {
                    "inputs": [
                        {
                            "name": "only_argument",
                            "type": 1,
                            "typeArguments": null
                        }
                    ],
                    "name": "takes_ints_returns_bool",
                    "output": {
                        "name": "",
                        "type": 0,
                        "typeArguments": null
                    }
                }
            ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;
    //`SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_ints_returns_bool(42_u32);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("000000009593586c000000000000002a", encoded);
    // ANCHOR_END: bindings_from_inline_contracts
    Ok(())
}

#[tokio::test]
async fn compile_bindings_array_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "[u16; 3]",
                "components": [
                  {
                    "name": "__array_element",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "u16",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "arg",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "name": "takes_array",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let input = [1, 2, 3];
    let call_handler = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "00000000101cbeb5000000000000000100000000000000020000000000000003",
        encoded
    );
}

#[tokio::test]
async fn compile_bindings_bool_array_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "[bool; 3]",
                "components": [
                  {
                    "name": "__array_element",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "bool",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "arg",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "name": "takes_array",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let input = [true, false, true];
    let call_handler = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "000000000c228226000000000000000100000000000000000000000000000001",
        encoded
    );
}

#[tokio::test]
async fn compile_bindings_byte_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "byte",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "arg",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "name": "takes_byte",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_byte(Byte(10u8));

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("00000000a4bd3861000000000000000a", encoded);
}

#[tokio::test]
async fn compile_bindings_string_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "str[23]",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "arg",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "name": "takes_string",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    // ANCHOR: contract_takes_string
    let call_handler = contract_instance.takes_string(
        "This is a full sentence"
            .try_into()
            .expect("failed to convert string into SizedAsciiString"),
    );
    // ANCHOR_END: contract_takes_string

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "00000000d56e76515468697320697320612066756c6c2073656e74656e636500",
        encoded
    );
}

#[tokio::test]
async fn compile_bindings_b256_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "b256",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "arg",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "name": "takes_b256",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    // ANCHOR: 256_arg
    let arg: [u8; 32] = hasher.finalize().into();

    let call_handler = contract_instance.takes_b256(Bits256(arg));
    // ANCHOR_END: 256_arg

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "0000000054992852d5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b",
        encoded
    );
}

#[tokio::test]
async fn compile_bindings_struct_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "[u8; 2]",
                "components": [
                  {
                    "name": "__array_element",
                    "type": 4,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "str[4]",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "struct MyStruct",
                "components": [
                  {
                    "name": "foo",
                    "type": 1,
                    "typeArguments": null
                  },
                  {
                    "name": "bar",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 4,
                "type": "u8",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "value",
                    "type": 3,
                    "typeArguments": null
                  }
                ],
                "name": "takes_struct",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );
    // Because of the abigen! macro, `MyStruct` is now in scope
    // and can be used!
    let input = MyStruct {
        foo: [10, 2],
        bar: "fuel".try_into().unwrap(),
    };

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "000000008d4ab9b0000000000000000a00000000000000026675656c00000000",
        encoded
    );
}

#[tokio::test]
async fn compile_bindings_nested_struct_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "bool",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "struct InnerStruct",
                "components": [
                  {
                    "name": "a",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "struct MyNestedStruct",
                "components": [
                  {
                    "name": "x",
                    "type": 4,
                    "typeArguments": null
                  },
                  {
                    "name": "foo",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 4,
                "type": "u16",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "top_value",
                    "type": 3,
                    "typeArguments": null
                  }
                ],
                "name": "takes_nested_struct",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    let inner_struct = InnerStruct { a: true };

    let input = MyNestedStruct {
        x: 10,
        foo: inner_struct,
    };

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_nested_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("0000000088bf8a1b000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn compile_bindings_enum_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "bool",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "enum MyEnum",
                "components": [
                  {
                    "name": "X",
                    "type": 3,
                    "typeArguments": null
                  },
                  {
                    "name": "Y",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "u32",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "my_enum",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "name": "takes_enum",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    let variant = MyEnum::X(42);

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_enum(variant);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );
    let expected = "0000000021b2784f0000000000000000000000000000002a";
    assert_eq!(encoded, expected);
}

#[allow(clippy::blacklisted_name)]
#[tokio::test]
async fn create_struct_from_decoded_tokens() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "bool",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "struct MyStruct",
                "components": [
                  {
                    "name": "foo",
                    "type": 3,
                    "typeArguments": null
                  },
                  {
                    "name": "bar",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "u8",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "my_val",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "name": "takes_struct",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    // Decoded tokens
    let foo = Token::U8(10);
    let bar = Token::Bool(true);

    // Create the struct using the decoded tokens.
    // `struct_from_tokens` is of type `MyStruct`.
    let struct_from_tokens = MyStruct::from_token(Token::Struct(vec![foo, bar]))?;

    assert_eq!(10, struct_from_tokens.foo);
    assert!(struct_from_tokens.bar);

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_struct(struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("00000000cb0b2f05000000000000000a0000000000000001", encoded);
    Ok(())
}

#[tokio::test]
async fn create_nested_struct_from_decoded_tokens() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        {
            "types": [
              {
                "typeId": 0,
                "type": "()",
                "components": [],
                "typeParameters": null
              },
              {
                "typeId": 1,
                "type": "bool",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "struct InnerStruct",
                "components": [
                  {
                    "name": "a",
                    "type": 1,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "struct MyNestedStruct",
                "components": [
                  {
                    "name": "x",
                    "type": 4,
                    "typeArguments": null
                  },
                  {
                    "name": "foo",
                    "type": 2,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 4,
                "type": "u16",
                "components": null,
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "inputs": [
                  {
                    "name": "top_value",
                    "type": 3,
                    "typeArguments": null
                  }
                ],
                "name": "takes_nested_struct",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    );

    // Creating just the InnerStruct is possible
    let a = Token::Bool(true);
    let inner_struct_token = Token::Struct(vec![a.clone()]);
    let inner_struct_from_tokens = InnerStruct::from_token(inner_struct_token.clone())?;
    assert!(inner_struct_from_tokens.a);

    // Creating the whole nested struct `MyNestedStruct`
    // from tokens.
    // `x` is the token for the field `x` in `MyNestedStruct`
    // `a` is the token for the field `a` in `InnerStruct`
    let x = Token::U16(10);

    let nested_struct_from_tokens =
        MyNestedStruct::from_token(Token::Struct(vec![x, inner_struct_token]))?;

    assert_eq!(10, nested_struct_from_tokens.x);
    assert!(nested_struct_from_tokens.foo.a);

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

    let call_handler = contract_instance.takes_nested_struct(nested_struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("0000000088bf8a1b000000000000000a0000000000000001", encoded);
    Ok(())
}

#[tokio::test]
async fn type_safe_output_values() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_output_test"
    );

    // `response`'s type matches the return type of `is_event()`
    let response = contract_instance.is_even(10).call().await?;
    assert!(response.value);

    // `response`'s type matches the return type of `return_my_string()`
    let response = contract_instance
        .return_my_string("fuel".try_into().unwrap())
        .call()
        .await?;

    assert_eq!(response.value, "fuel");

    let my_struct = MyStruct { foo: 10, bar: true };

    let response = contract_instance.return_my_struct(my_struct).call().await?;

    assert_eq!(response.value.foo, 10);
    assert!(response.value.bar);
    Ok(())
}

#[tokio::test]
async fn call_with_structs() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    // ANCHOR: struct_generation
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/complex_types_contract/out/debug/complex_types_contract-abi.json"
    );

    // Here we can use `CounterConfig`, a struct originally
    // defined in the Sway contract.
    let counter_config = CounterConfig {
        dummy: true,
        initial_value: 42,
    };
    // ANCHOR_END: struct_generation

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/complex_types_contract/out/debug/complex_types_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let response = contract_instance
        .initialize_counter(counter_config) // Build the ABI call
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);

    let response = contract_instance.increment_counter(10).call().await?;

    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn call_with_empty_return() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/call_empty_return"
    );

    let _response = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    Ok(())
}

#[tokio::test]
async fn abigen_different_structs_same_arg_name() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/two_structs"
    );

    let param_one = StructOne { foo: 42 };
    let param_two = StructTwo { bar: 42 };

    let res_one = contract_instance.something(param_one).call().await?;

    assert_eq!(res_one.value, 43);

    let res_two = contract_instance.something_else(param_two).call().await?;

    assert_eq!(res_two.value, 41);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/revert_transaction_error"
    );

    let response = contract_instance.make_transaction_fail(0).call().await;

    assert!(matches!(response, Err(Error::RevertTransactionError(..))));
    Ok(())
}

#[tokio::test]
async fn multiple_read_calls() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/multiple_read_calls"
    );

    contract_instance.store(42).call().await?;

    // Use "simulate" because the methods don't actually run a transaction, but just a dry-run
    // We can notice here that, thanks to this, we don't generate a TransactionId collision,
    // even if the transactions are theoretically the same.
    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);

    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/empty_arguments"
    );

    let response = contract_instance
        .method_with_empty_argument()
        .call()
        .await?;
    assert_eq!(response.value, 63);
    Ok(())
}

#[tokio::test]
async fn test_large_return_data() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/large_return_data"
    );

    let res = contract_instance.get_id().call().await?;

    assert_eq!(
        res.value.0,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );

    // One word-sized string
    let res = contract_instance.get_small_string().call().await?;
    assert_eq!(res.value, "gggggggg");

    // Two word-sized string
    let res = contract_instance.get_large_string().call().await?;
    assert_eq!(res.value, "ggggggggg");

    // Large struct will be bigger than a `WORD`.
    let res = contract_instance.get_large_struct().call().await?;
    assert_eq!(res.value.foo, 12);
    assert_eq!(res.value.bar, 42);

    // Array will be returned in `ReturnData`.
    let res = contract_instance.get_large_array().call().await?;
    assert_eq!(res.value, [1, 2]);

    let res = contract_instance.get_contract_id().call().await?;

    // First `value` is from `CallResponse`.
    // Second `value` is from Sway `ContractId` type.
    assert_eq!(
        res.value,
        ContractId::from([
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ])
    );
    Ok(())
}

#[tokio::test]
async fn test_provider_launch_and_connect() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = WalletUnlocked::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, vec![], None).await;
    let connected_provider = Provider::connect(address.to_string()).await?;

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected =
        MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let response = contract_instance_connected
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    assert_eq!(42, response.value);

    wallet.set_provider(launched_provider);
    let contract_instance_launched =
        MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let response = contract_instance_launched
        .increment_counter(10)
        .call()
        .await?;
    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_contract_calling_contract() -> Result<(), Error> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    // Load and deploy the first compiled contract
    setup_contract_test!(
        foo_contract_instance,
        shared_wallet,
        "packages/fuels/tests/test_projects/foo_contract"
    );
    let foo_contract_id = foo_contract_instance._get_contract_id();

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.foo(true).call().await?;
    assert!(!res.value);

    // Load and deploy the second compiled contract
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/test_projects/foo_caller_contract"
    );

    // Calls the contract that calls the `FooContract` contract, also just
    // flips the bool value passed to it.
    // ANCHOR: external_contract
    let bits = *foo_contract_id.hash();
    let res = foo_caller_contract_instance
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert!(!res.value);
    Ok(())
}

#[tokio::test]
async fn test_contract_setup_macro_deploy_with_salt() -> Result<(), Error> {
    // ANCHOR: contract_setup_macro_multi
    // The first wallet name must be `shared_wallet`
    setup_contract_test!(
        foo_contract_instance,
        shared_wallet,
        "packages/fuels/tests/test_projects/foo_contract"
    );
    let foo_contract_id = foo_contract_instance._get_contract_id();

    // The macros that want to use the `shared_wallet` have to set
    // the wallet name to `None`
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/test_projects/foo_caller_contract"
    );
    let foo_caller_contract_id = foo_caller_contract_instance._get_contract_id();

    setup_contract_test!(
        foo_caller_contract_instance2,
        None,
        "packages/fuels/tests/test_projects/foo_caller_contract"
    );
    let foo_caller_contract_id2 = foo_caller_contract_instance2._get_contract_id();

    // Because we deploy with salt, we can deploy the same contract multiple times
    assert_ne!(foo_caller_contract_id, foo_caller_contract_id2);

    // The first contract can be called because they were deployed on the same provider
    let bits = *foo_contract_id.hash();
    let res = foo_caller_contract_instance
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    assert!(!res.value);

    let res = foo_caller_contract_instance2
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    assert!(!res.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_gas_errors() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 1;
    let amount_per_coin = 1_000_000;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    // Test running out of gas. Gas price as `None` will be 0.
    let gas_limit = 100;
    let contract_instace_call = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(gas_limit), None));

    //  Test that the call will use more gas than the gas limit
    let gas_used = contract_instace_call
        .estimate_transaction_cost(None)
        .await?
        .gas_used;
    assert!(gas_used > gas_limit);

    let response = contract_instace_call
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));

    // Test for insufficient base asset amount to pay for the transaction fee
    let response = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(Some(100_000_000_000), None, None))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: Response errors; enough coins could not be found";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_call_param_gas_errors() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let response = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1000), None))
        .call_params(CallParameters::new(None, None, Some(1)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Revert transaction error: OutOfGas, receipts:";
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1), None))
        .call_params(CallParameters::new(None, None, Some(1000)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_amount_and_asset_forwarding() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/token_ops"
    );
    let contract_id = contract_instance._get_contract_id();

    let mut balance_response = contract_instance
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 0);

    contract_instance.mint_coins(5_000_000).call().await?;

    balance_response = contract_instance
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = contract_instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    contract_instance
        .transfer_coins_to_output(1_000_000, contract_id.into(), address.into())
        .append_variable_outputs(1)
        .call()
        .await?;

    let asset_id = AssetId::from(*contract_id.hash());
    let call_params = CallParameters::new(Some(0), Some(asset_id), None);
    let tx_params = TxParameters::new(None, Some(1_000_000), None);

    let response = contract_instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*contract_id.hash())
    );
    Ok(())
}

#[tokio::test]
async fn test_multiple_args() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    // Make sure we can call the contract with multiple arguments
    let response = contract_instance.get(5, 6).call().await?;

    assert_eq!(response.value, 5);

    let t = MyType { x: 5, y: 6 };
    let response = contract_instance.get_alt(t.clone()).call().await?;
    assert_eq!(response.value, t);

    let response = contract_instance.get_single(5).call().await?;
    assert_eq!(response.value, 5);
    Ok(())
}

#[tokio::test]
async fn test_tuples() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/tuples"
    );

    {
        let response = contract_instance.returns_tuple((1, 2)).call().await?;

        assert_eq!(response.value, (1, 2));
    }
    {
        // Tuple with struct.
        let my_struct_tuple = (
            42,
            Person {
                name: "Jane".try_into()?,
            },
        );
        let response = contract_instance
            .returns_struct_in_tuple(my_struct_tuple.clone())
            .call()
            .await?;

        assert_eq!(response.value, my_struct_tuple);
    }
    {
        // Tuple with enum.
        let my_enum_tuple: (u64, State) = (42, State::A());

        let response = contract_instance
            .returns_enum_in_tuple(my_enum_tuple.clone())
            .call()
            .await?;

        assert_eq!(response.value, my_enum_tuple);
    }
    {
        // Tuple with single element
        let my_enum_tuple = (123u64,);

        let response = contract_instance
            .single_element_tuple(my_enum_tuple)
            .call()
            .await?;

        assert_eq!(response.value, my_enum_tuple);
    }
    {
        // tuple with b256
        let id = *ContractId::zeroed();
        let my_b256_u8_tuple = (Bits256(id), 10);

        let response = contract_instance
            .tuple_with_b256(my_b256_u8_tuple)
            .call()
            .await?;

        assert_eq!(response.value, my_b256_u8_tuple);
    }

    Ok(())
}

#[tokio::test]
async fn test_array() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    assert_eq!(
        contract_instance.get_array([42; 2]).call().await?.value,
        [42; 2]
    );
    Ok(())
}

#[tokio::test]
async fn test_arrays_with_custom_types() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let persons = [
        Person {
            name: "John".try_into()?,
        },
        Person {
            name: "Jane".try_into()?,
        },
    ];

    let response = contract_instance.array_of_structs(persons).call().await?;

    assert_eq!("John", response.value[0].name);
    assert_eq!("Jane", response.value[1].name);

    let states = [State::A(), State::B()];

    let response = contract_instance
        .array_of_enums(states.clone())
        .call()
        .await?;

    assert_eq!(states[0], response.value[0]);
    assert_eq!(states[1], response.value[1]);
    Ok(())
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/auth_testing_contract"
    );

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = contract_instance
        .check_msg_sender(wallet.address().into())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}

#[tokio::test]
async fn workflow_enum_inside_struct() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/enum_inside_struct"
    );

    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(222),
        glass: 333,
    };

    let response = contract_instance
        .return_enum_inside_struct(11)
        .call()
        .await?;

    assert_eq!(response.value, expected);

    let enum_inside_struct = Cocktail {
        the_thing_you_mix_in: Shaker::Cosmopolitan(444),
        glass: 555,
    };

    let response = contract_instance
        .take_enum_inside_struct(enum_inside_struct)
        .call()
        .await?;

    assert_eq!(response.value, 6666);
    Ok(())
}

#[tokio::test]
async fn test_logd_receipts() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_logdata"
    );

    let mut value = [0u8; 32];
    value[0] = 0xFF;
    value[1] = 0xEE;
    value[2] = 0xDD;
    value[12] = 0xAA;
    value[13] = 0xBB;
    value[14] = 0xCC;

    let response = contract_instance
        .use_logd_opcode(Bits256(value), 3, 6)
        .call()
        .await?;
    assert_eq!(response.logs, vec!["ffeedd", "ffeedd000000"]);

    let response = contract_instance
        .use_logd_opcode(Bits256(value), 14, 15)
        .call()
        .await?;
    assert_eq!(
        response.logs,
        vec![
            "ffeedd000000000000000000aabb",
            "ffeedd000000000000000000aabbcc"
        ]
    );

    let response = contract_instance.dont_use_logd().call().await?;
    assert!(response.logs.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_single_asset() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);

    for (_utxo_id, coin) in coins {
        let balance = wallet.get_asset_balance(&coin.asset_id).await;
        assert_eq!(balance?, number_of_coins * amount_per_coin);
    }

    let balances = wallet.get_balances().await?;
    let expected_key = format!("{:#x}", BASE_ASSET_ID);
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&expected_key));
    assert_eq!(
        *balances.get(&expected_key).unwrap(),
        number_of_coins * amount_per_coin
    );

    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_multi_asset() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_assets = 7;
    let coins_per_asset = 21;
    let amount_per_coin = 11;
    let (coins, asset_ids) = setup_multiple_assets_coins(
        wallet.address(),
        number_of_assets,
        coins_per_asset,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);
    let balances = wallet.get_balances().await?;
    assert_eq!(balances.len() as u64, number_of_assets);

    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);

        let expected_key = format!("{:#x}", asset_id);
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            coins_per_asset * amount_per_coin
        );
    }

    Ok(())
}

#[tokio::test]
async fn sway_native_types_support() -> Result<(), Box<dyn std::error::Error>> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/sway_native_types"
    );

    let user = User {
        weight: 10,
        address: Address::zeroed(),
    };

    let response = contract_instance.wrapped_address(user).call().await?;

    assert_eq!(response.value.address, Address::zeroed());

    let response = contract_instance
        .unwrapped_address(Address::zeroed())
        .call()
        .await?;

    assert_eq!(
        response.value,
        Address::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")?
    );
    Ok(())
}

#[tokio::test]
async fn test_transaction_script_workflow() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let call_handler = contract_instance.initialize_counter(42);

    let script = call_handler.get_call_execution_script().await?;
    assert!(script.tx.is_script());

    let provider = wallet.get_provider()?;
    let receipts = script.call(provider).await?;

    let response = call_handler.get_response(receipts)?;
    assert_eq!(response.value, 42);
    Ok(())
}

#[tokio::test]
async fn enum_coding_w_variable_width_variants() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/enum_encoding"
    );

    // If we had a regression on the issue of enum encoding width, then we'll
    // probably end up mangling arg_2 and onward which will fail this test.
    let expected = BigBundle {
        arg_1: EnumThatHasABigAndSmallVariant::Small(12345),
        arg_2: 6666,
        arg_3: 7777,
        arg_4: 8888,
    };
    let actual = contract_instance.get_big_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_instance
        .check_big_bundle_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the bundle correctly. Investigate!"
    );
    Ok(())
}

#[tokio::test]
async fn enum_coding_w_unit_enums() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/enum_encoding"
    );

    // If we had a regression on the issue of unit enum encoding width, then
    // we'll end up mangling arg_2
    let expected = UnitBundle {
        arg_1: UnitEnum::var2(),
        arg_2: u64::MAX,
    };
    let actual = contract_instance.get_unit_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_instance
        .check_unit_bundle_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the bundle correctly. Investigate!"
    );
    Ok(())
}

#[tokio::test]
async fn enum_as_input() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/enum_as_input"
    );

    let expected = StandardEnum::Two(12345);
    let actual = contract_instance.get_standard_enum().call().await?.value;
    assert_eq!(expected, actual);

    let fuelvm_judgement = contract_instance
        .check_standard_enum_integrity(expected)
        .call()
        .await?
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the standard enum correctly. Investigate!"
    );

    let expected = UnitEnum::Two();
    let actual = contract_instance.get_unit_enum().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_instance
        .check_unit_enum_integrity(expected)
        .call()
        .await?
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the unit enum correctly. Investigate!"
    );
    Ok(())
}

#[tokio::test]
async fn nested_structs() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/nested_structs"
    );

    let expected = AllStruct {
        some_struct: SomeStruct { par_1: 12345 },
    };

    let actual = contract_instance.get_struct().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_instance
        .check_struct_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the argument correctly. Investigate!"
    );

    let memory_address = MemoryAddress {
        contract_id: ContractId::zeroed(),
        function_selector: 10,
        function_data: 0,
    };

    let call_data = CallData {
        memory_address,
        num_coins_to_forward: 10,
        asset_id_of_coins_to_forward: ContractId::zeroed(),
        amount_of_gas_to_forward: 5,
    };

    let actual = contract_instance
        .nested_struct_with_reserved_keyword_substring(call_data.clone())
        .call()
        .await?
        .value;

    assert_eq!(actual, call_data);
    Ok(())
}

#[tokio::test]
async fn test_multi_call() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let call_handler_1 = contract_instance.initialize_counter(42);
    let call_handler_2 = contract_instance.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let (counter, array): (u64, [u64; 2]) = multi_call_handler.call().await?.value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);
    Ok(())
}

#[tokio::test]
async fn test_multi_call_script_workflow() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let call_handler_1 = contract_instance.initialize_counter(42);
    let call_handler_2 = contract_instance.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let provider = &wallet.get_provider()?;
    let script = multi_call_handler.get_call_execution_script().await?;
    let receipts = script.call(provider).await.unwrap();
    let (counter, array) = multi_call_handler
        .get_response::<(u64, [u64; 2])>(receipts)?
        .value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);
    Ok(())
}

#[tokio::test]
async fn test_storage_initialization() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: storage_slot_create
    let key = Bytes32::from([1u8; 32]);
    let value = Bytes32::from([2u8; 32]);
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];
    // ANCHOR_END: storage_slot_create

    // ANCHOR: manual_storage
    let contract_id = Contract::deploy_with_parameters(
        "tests/test_projects/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([0; 32]),
    )
    .await?;
    // ANCHOR_END: manual_storage

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let result = contract_instance
        .get_value_b256(Bits256(key.into()))
        .call()
        .await?
        .value;
    assert_eq!(result.0, *value);

    Ok(())
}

#[tokio::test]
async fn can_use_try_into_to_construct_struct_from_bytes() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/enum_inside_struct/out/debug\
        /enum_inside_struct-abi.json"
    );
    let cocktail_in_bytes: Vec<u8> = vec![
        0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3,
    ];

    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(2),
        glass: 3,
    };

    // as slice
    let actual: Cocktail = cocktail_in_bytes[..].try_into()?;
    assert_eq!(actual, expected);

    // as ref
    let actual: Cocktail = (&cocktail_in_bytes).try_into()?;
    assert_eq!(actual, expected);

    // as value
    let actual: Cocktail = cocktail_in_bytes.try_into()?;
    assert_eq!(actual, expected);
    Ok(())
}

#[tokio::test]
async fn can_use_try_into_to_construct_enum_from_bytes() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/enum_inside_struct/out/debug\
        /enum_inside_struct-abi.json"
    );
    // ANCHOR: manual_decode
    let shaker_in_bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];

    let expected = Shaker::Mojito(2);

    // as slice
    let actual: Shaker = shaker_in_bytes[..].try_into()?;
    assert_eq!(actual, expected);

    // as ref
    let actual: Shaker = (&shaker_in_bytes).try_into()?;
    assert_eq!(actual, expected);

    // as value
    let actual: Shaker = shaker_in_bytes.try_into()?;
    assert_eq!(actual, expected);
    // ANCHOR_END: manual_decode
    Ok(())
}

#[tokio::test]
async fn type_inside_enum() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/type_inside_enum"
    );

    // String inside enum
    let enum_string = SomeEnum::SomeStr("asdf".try_into()?);
    let response = contract_instance
        .str_inside_enum(enum_string.clone())
        .call()
        .await?;
    assert_eq!(response.value, enum_string);

    // Array inside enum
    let enum_array = SomeEnum::SomeArr([1, 2, 3, 4, 5, 6, 7]);
    let response = contract_instance
        .arr_inside_enum(enum_array.clone())
        .call()
        .await?;
    assert_eq!(response.value, enum_array);

    // Struct inside enum
    let response = contract_instance
        .return_struct_inside_enum(11)
        .call()
        .await?;
    let expected = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 99 });
    assert_eq!(response.value, expected);
    let struct_inside_enum = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 66 });
    let response = contract_instance
        .take_struct_inside_enum(struct_inside_enum)
        .call()
        .await?;
    assert_eq!(response.value, 8888);

    // Enum inside enum
    let expected_enum = EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42)));
    let response = contract_instance.get_nested_enum().call().await?;
    assert_eq!(response.value, expected_enum);

    let response = contract_instance
        .check_nested_enum_integrity(expected_enum)
        .call()
        .await?;
    assert!(
        response.value,
        "The FuelVM deems that we've not encoded the nested enum correctly. Investigate!"
    );
    Ok(())
}

#[tokio::test]
async fn test_init_storage_automatically() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: automatic_storage
    let contract_id = Contract::deploy_with_parameters(
        "tests/test_projects/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(
            Some("tests/test_projects/contract_storage_test/out/debug/contract_storage_test-storage_slots.json".to_string())),
        Salt::default(),
    )
        .await?;
    // ANCHOR_END: automatic_storage

    let key1 =
        Bytes32::from_str("de9090cb50e71c2588c773487d1da7066d0c719849a7e58dc8b6397a25c567c0")
            .unwrap();
    let key2 =
        Bytes32::from_str("f383b0ce51358be57daa3b725fe44acdb2d880604e367199080b4379c41bb6ed")
            .unwrap();

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let value = contract_instance
        .get_value_b256(Bits256(*key1))
        .call()
        .await?
        .value;
    assert_eq!(value.0, [1u8; 32]);

    let value = contract_instance
        .get_value_u64(Bits256(*key2))
        .call()
        .await?
        .value;
    assert_eq!(value, 64);
    Ok(())
}

#[tokio::test]
async fn test_init_storage_automatically_bad_json_path() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let response = Contract::deploy_with_parameters(
        "tests/test_projects/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(
            Some("tests/test_projects/contract_storage_test/out/debug/contract_storage_test-storage_slts.json".to_string())),
        Salt::default(),
    ).await.expect_err("Should fail");

    let expected = "Invalid data:";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn contract_method_call_respects_maturity() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/transaction_block_height"
    );

    let call_w_maturity = |call_maturity| {
        let mut prepared_call = contract_instance.calling_this_will_produce_a_block();
        prepared_call.tx_parameters.maturity = call_maturity;
        prepared_call.call()
    };

    call_w_maturity(1).await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");
    Ok(())
}

#[tokio::test]
async fn contract_deployment_respects_maturity() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/transaction_block_height/out/debug/transaction_block_height-abi.json"
    );

    let config = Config {
        manual_blocks_enabled: true,
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    let deploy_w_maturity = |maturity| {
        let parameters = TxParameters {
            maturity,
            ..TxParameters::default()
        };
        Contract::deploy(
            "tests/test_projects/transaction_block_height/out/debug/transaction_block_height.bin",
            wallet,
            parameters,
            StorageConfiguration::default(),
        )
    };

    let err = deploy_w_maturity(1).await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(
        err,
        Error::ValidationError(fuel_gql_client::fuel_tx::ValidationError::TransactionMaturity)
    ));

    provider.produce_blocks(1).await?;
    deploy_w_maturity(1)
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");
    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<(), Error> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0);

    provider.produce_blocks(3).await?;

    assert_eq!(provider.latest_block_height().await?, 3);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

#[tokio::test]
async fn can_handle_sway_function_called_new() -> anyhow::Result<()> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/collision_in_fn_names"
    );

    let response = contract_instance.new().call().await?.value;

    assert_eq!(response, 12345);
    Ok(())
}

async fn setup_predicate_test(
    file_path: &str,
    num_coins: u64,
    coin_amount: u64,
) -> Result<(Predicate, WalletUnlocked, WalletUnlocked, AssetId), Error> {
    let predicate = Predicate::load_from(file_path)?;

    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(2), Some(num_coins), Some(coin_amount)),
        Some(Config {
            predicates: true,
            utxo_validation: true,
            ..Config::local_node()
        }),
    )
    .await;

    let sender = wallets.pop().unwrap();
    let receiver = wallets.pop().unwrap();
    let asset_id = AssetId::default();

    Ok((predicate, sender, receiver, asset_id))
}

#[tokio::test]
async fn predicate_with_multiple_coins() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_true/out/debug/predicate_true.bin",
        3,
        100,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 10;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 300);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate - 1,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 10);
    Ok(())
}

#[tokio::test]
async fn can_call_no_arg_predicate_returns_true() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_true/out/debug/predicate_true.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 2;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_no_arg_predicate_returns_false() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_false/out/debug/predicate_false.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 4;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_u32_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_u32/out/debug/predicate_u32.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 8;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    // invalid predicate data
    let predicate_data = ABIEncoder::encode(&[101_u32.into_token()]).unwrap();
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);

    // valid predicate data
    let predicate_data = ABIEncoder::encode(&[1078_u32.into_token()]).unwrap();
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_address_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_address/out/debug/predicate_address.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 16;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    let addr =
        Address::from_str("0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")
            .unwrap();
    let predicate_data = ABIEncoder::encode(&[addr.into_token()]).unwrap();
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_struct_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/test_projects/predicate_struct/out/debug/predicate_struct.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 8;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    // invalid predicate data
    let predicate_data = ABIEncoder::encode(&[true.into_token(), 55_u32.into_token()]).unwrap();
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);

    // valid predicate data
    let predicate_data = ABIEncoder::encode(&[true.into_token(), 100_u32.into_token()]).unwrap();
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn test_get_gas_used() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let gas_used = contract_instance
        .initialize_counter(42)
        .call()
        .await?
        .gas_used;

    assert!(gas_used > 0);
    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    assert_eq!(contract_instance._get_wallet().address(), wallet.address());
    //`_get_contract_id()` is tested in
    // async fn test_contract_calling_contract() -> Result<(), Error> {
    Ok(())
}

#[tokio::test]
async fn test_network_error() -> Result<(), anyhow::Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = WalletUnlocked::new_random(None);

    let config = CoreConfig::local_node();
    let service = FuelService::new_node(config).await?;
    let provider = Provider::connect(service.bound_address.to_string()).await?;

    wallet.set_provider(provider);

    // Simulate an unreachable node
    service.stop().await;

    let response = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await;

    assert!(matches!(response, Err(Error::ProviderError(_))));
    Ok(())
}

#[tokio::test]
async fn str_in_array() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/str_in_array"
    );

    let input = ["foo", "bar", "baz"].map(|str| str.try_into().unwrap());
    let response = contract_instance
        .take_array_string_shuffle(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["baz", "foo", "bar"]);

    let response = contract_instance
        .take_array_string_return_single(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["foo"]);

    let response = contract_instance
        .take_array_string_return_single_element(input)
        .call()
        .await?;

    assert_eq!(response.value, "bar");
    Ok(())
}

#[tokio::test]
#[should_panic(
    expected = "SizedAsciiString<4> can only be constructed from a String of length 4. Got: fuell"
)]
async fn strings_must_have_correct_length() {
    abigen!(
        SimpleContract,
        r#"
        {
          "types": [
            {
              "typeId": 0,
              "type": "()",
              "components": [],
              "typeParameters": null
            },
            {
              "typeId": 1,
              "type": "str[4]",
              "components": null,
              "typeParameters": null
            }
          ],
          "functions": [
            {
              "inputs": [
                {
                  "name": "arg",
                  "type": 1,
                  "typeArguments": null
                }
              ],
              "name": "takes_string",
              "output": {
                "name": "",
                "type": 0,
                "typeArguments": null
              }
            }
          ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();
    let _ = contract_instance.takes_string("fuell".try_into().unwrap());
}

#[tokio::test]
#[should_panic(
    expected = "SizedAsciiString must be constructed from a string containing only ascii encodable characters. Got: fueŁ"
)]
async fn strings_must_have_all_ascii_chars() {
    abigen!(
        SimpleContract,
        r#"
        {
          "types": [
            {
              "typeId": 0,
              "type": "()",
              "components": [],
              "typeParameters": null
            },
            {
              "typeId": 1,
              "type": "str[4]",
              "components": null,
              "typeParameters": null
            }
          ],
          "functions": [
            {
              "inputs": [
                {
                  "name": "arg",
                  "type": 1,
                  "typeArguments": null
                }
              ],
              "name": "takes_string",
              "output": {
                "name": "",
                "type": 0,
                "typeArguments": null
              }
            }
          ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();
    let _ = contract_instance.takes_string("fueŁ".try_into().unwrap());
}

#[tokio::test]
#[should_panic(
    expected = "SizedAsciiString<4> can only be constructed from a String of length 4. Got: fuell"
)]
async fn strings_must_have_correct_length_custom_types() {
    abigen!(
        SimpleContract,
        r#"
        {
          "types": [
            {
              "typeId": 0,
              "type": "()",
              "components": [],
              "typeParameters": null
            },
            {
              "typeId": 1,
              "type": "[_; 2]",
              "components": [
                {
                  "name": "__array_element",
                  "type": 4,
                  "typeArguments": null
                }
              ],
              "typeParameters": null
            },
            {
              "typeId": 2,
              "type": "enum MyEnum",
              "components": [
                {
                  "name": "foo",
                  "type": 1,
                  "typeArguments": null
                },
                {
                  "name": "bar",
                  "type": 3,
                  "typeArguments": null
                }
              ],
              "typeParameters": null
            },
            {
              "typeId": 3,
              "type": "str[4]",
              "components": null,
              "typeParameters": null
            },
            {
              "typeId": 4,
              "type": "u8",
              "components": null,
              "typeParameters": null
            }
          ],
          "functions": [
            {
              "inputs": [
                {
                  "name": "value",
                  "type": 2,
                  "typeArguments": null
                }
              ],
              "name": "takes_enum",
              "output": {
                "name": "",
                "type": 0,
                "typeArguments": null
              }
            }
          ]
        }
        "#,
    );

    let wallet = launch_provider_and_get_wallet().await;
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();
    let _ = contract_instance.takes_enum(MyEnum::bar("fuell".try_into().unwrap()));
}

#[tokio::test]
#[should_panic(
    expected = "SizedAsciiString must be constructed from a string containing only ascii encodable characters. Got: fueŁ"
)]
async fn strings_must_have_all_ascii_chars_custom_types() {
    abigen!(
        SimpleContract,
        r#"
        {
          "types": [
            {
              "typeId": 0,
              "type": "()",
              "components": [],
              "typeParameters": null
            },
            {
              "typeId": 1,
              "type": "str[4]",
              "components": null,
              "typeParameters": null
            },
            {
              "typeId": 2,
              "type": "struct InnerStruct",
              "components": [
                {
                  "name": "bar",
                  "type": 1,
                  "typeArguments": null
                }
              ],
              "typeParameters": null
            },
            {
              "typeId": 3,
              "type": "struct MyNestedStruct",
              "components": [
                {
                  "name": "x",
                  "type": 4,
                  "typeArguments": null
                },
                {
                  "name": "foo",
                  "type": 2,
                  "typeArguments": null
                }
              ],
              "typeParameters": null
            },
            {
              "typeId": 4,
              "type": "u16",
              "components": null,
              "typeParameters": null
            }
          ],
          "functions": [
            {
              "inputs": [
                {
                  "name": "top_value",
                  "type": 3,
                  "typeArguments": null
                }
              ],
              "name": "takes_nested_struct",
              "output": {
                "name": "",
                "type": 0,
                "typeArguments": null
              }
            }
          ]
        }
        "#,
    );

    let inner_struct = InnerStruct {
        bar: "fueŁ".try_into().unwrap(),
    };
    let input = MyNestedStruct {
        x: 10,
        foo: inner_struct,
    };

    let wallet = launch_provider_and_get_wallet().await;
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();
    let _ = contract_instance.takes_nested_struct(input);
}

#[tokio::test]
async fn test_connect_wallet() -> anyhow::Result<()> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet_1 = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    let id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet_1,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    // pay for call with wallet_1
    let contract_instance = MyContractBuilder::new(id.to_string(), wallet_1.clone()).build();
    let tx_params = TxParameters::new(Some(10), Some(10000), None);
    contract_instance
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm that funds have been deducted
    let wallet_1_balance = wallet_1.get_asset_balance(&Default::default()).await?;
    assert!(DEFAULT_COIN_AMOUNT > wallet_1_balance);

    // pay for call with wallet_2
    contract_instance
        ._with_wallet(wallet_2.clone())?
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm there are no changes to wallet_1, wallet_2 has been charged
    let wallet_1_balance_second_call = wallet_1.get_asset_balance(&Default::default()).await?;
    let wallet_2_balance = wallet_2.get_asset_balance(&Default::default()).await?;
    assert_eq!(wallet_1_balance_second_call, wallet_1_balance);
    assert!(DEFAULT_COIN_AMOUNT > wallet_2_balance);
    Ok(())
}

#[tokio::test]
async fn contract_call_fee_estimation() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let gas_price = 100_000_000;
    let gas_limit = 800;
    let tolerance = 0.2;

    let expected_min_gas_price = 0; // This is the default min_gas_price from the ConsensusParameters
    let expected_gas_used = 710;
    let expected_metered_bytes_size = 720;
    let expected_total_fee = 359;

    let estimated_transaction_cost = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(Some(gas_price), Some(gas_limit), None))
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?;

    assert_eq!(
        estimated_transaction_cost.min_gas_price,
        expected_min_gas_price
    );
    assert_eq!(estimated_transaction_cost.gas_price, gas_price);
    assert_eq!(estimated_transaction_cost.gas_used, expected_gas_used);
    assert_eq!(
        estimated_transaction_cost.metered_bytes_size,
        expected_metered_bytes_size
    );
    assert_eq!(estimated_transaction_cost.total_fee, expected_total_fee);
    Ok(())
}

#[tokio::test]
async fn contract_call_has_same_estimated_and_used_gas() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let tolerance = 0.0;
    let estimated_gas_used = contract_instance
        .initialize_counter(42) // Build the ABI call
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?
        .gas_used;

    let gas_used = contract_instance
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?
        .gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn mutl_call_has_same_estimated_and_used_gas() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let call_handler_1 = contract_instance.initialize_counter(42);
    let call_handler_2 = contract_instance.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let tolerance = 0.0;
    let estimated_gas_used = multi_call_handler
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?
        .gas_used;

    let gas_used = multi_call_handler.call::<(u64, [u64; 2])>().await?.gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn testnet_hello_world() -> Result<(), Error> {
    // Note that this test might become flaky.
    // This test depends on:
    // 1. The testnet being up and running;
    // 2. The testnet address being the same as the one in the test;
    // 3. The hardcoded wallet having enough funds to pay for the transaction.
    // This is a nice test to showcase the SDK interaction with
    // the testnet. But, if it becomes too problematic, we should remove it.
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    // Create a provider pointing to the testnet.
    let provider = Provider::connect("node-beta-1.fuel.network").await.unwrap();

    // Setup the private key.
    let secret =
        SecretKey::from_str("a0447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568")
            .unwrap();

    // Create the wallet.
    let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

    dbg!(wallet.address().to_string());

    let params = TxParameters::new(Some(1), Some(2000), None);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        params,
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let response = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(params)
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);

    let response = contract_instance
        .increment_counter(10)
        .tx_params(params)
        .call()
        .await?;

    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_input_message() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compare_messages =
        |messages_from_provider: Vec<OtherMessage>, used_messages: Vec<Message>| -> bool {
            iter::zip(&used_messages, &messages_from_provider).all(|(a, b)| {
                a.sender == b.sender.0 .0
                    && a.recipient == b.recipient.0 .0
                    && a.owner == b.owner.0 .0
                    && a.nonce == b.nonce.0
                    && a.amount == b.amount.0
            })
        };

    let mut wallet = WalletUnlocked::new_random(None);

    let messages = setup_single_message(
        &Bech32Address {
            hrp: "".to_string(),
            hash: Default::default(),
        },
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0,
        vec![1, 2],
    );

    let (provider, _) = setup_test_provider(vec![], messages.clone(), None).await;
    wallet.set_provider(provider);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected =
        MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let messages_from_provider = wallet.get_messages().await?;
    assert!(compare_messages(messages_from_provider, messages));

    let response = contract_instance_connected
        .initialize_counter(42) // Build the ABI call
        .call()
        .await?;

    assert_eq!(42, response.value);

    Ok(())
}

#[tokio::test]
async fn generics_test() -> anyhow::Result<()> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/generics"
    );

    {
        // ANCHOR: generic
        // simple struct with a single generic param
        let arg1 = SimpleGeneric {
            single_generic_param: 123u64,
        };

        let result = contract_instance
            .struct_w_generic(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
        // ANCHOR_END: generic
    }
    {
        // struct that delegates the generic param internally
        let arg1 = PassTheGenericOn {
            one: SimpleGeneric {
                single_generic_param: "abc".try_into()?,
            },
        };

        let result = contract_instance
            .struct_delegating_generic(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // struct that has the generic in an array
        let arg1 = StructWArrayGeneric { a: [1u32, 2u32] };

        let result = contract_instance
            .struct_w_generic_in_array(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // struct that has the generic in a tuple
        let arg1 = StructWTupleGeneric { a: (1, 2) };

        let result = contract_instance
            .struct_w_generic_in_tuple(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // struct with generic in variant
        let arg1 = EnumWGeneric::b(10);
        let result = contract_instance
            .enum_w_generic(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // complex case
        let pass_through = PassTheGenericOn {
            one: SimpleGeneric {
                single_generic_param: "ab".try_into()?,
            },
        };
        let w_arr_generic = StructWArrayGeneric {
            a: [pass_through.clone(), pass_through],
        };

        let arg1 = MegaExample {
            a: ([Bits256([0; 32]), Bits256([0; 32])], "ab".try_into()?),
            b: (
                [EnumWGeneric::b(StructWTupleGeneric {
                    a: (w_arr_generic.clone(), w_arr_generic),
                })],
                10u32,
            ),
        };

        let result = contract_instance
            .complex_test(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }

    Ok(())
}

#[tokio::test]
async fn test_gas_forwarded_defaults_to_tx_limit() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let gas_limit = 225883;
    let response = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(gas_limit), None))
        .call()
        .await?;

    let gas_forwarded = response
        .receipts
        .iter()
        .find(|r| matches!(r, Receipt::Call { .. }))
        .unwrap()
        .gas()
        .unwrap();

    assert_eq!(gas_limit, gas_forwarded);

    Ok(())
}

#[tokio::test]
async fn test_rust_option_can_be_decoded() -> Result<(), Box<dyn std::error::Error>> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/options/out/debug/options-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/options/out/debug/options.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let expected_some_address = Some(expected_address);
    let response = contract_instance.get_some_address().call().await?;

    assert_eq!(response.value, expected_some_address);

    let expected_some_u64 = Some(10);
    let response = contract_instance.get_some_u64().call().await?;

    assert_eq!(response.value, expected_some_u64);

    let response = contract_instance.get_some_struct().call().await?;
    assert_eq!(response.value, Some(s.clone()));

    let response = contract_instance.get_some_enum().call().await?;
    assert_eq!(response.value, Some(e.clone()));

    let response = contract_instance.get_some_tuple().call().await?;
    assert_eq!(response.value, Some((s.clone(), e.clone())));

    let expected_none = None;
    let response = contract_instance.get_none().call().await?;

    assert_eq!(response.value, expected_none);

    Ok(())
}

#[tokio::test]
async fn test_rust_option_can_be_encoded() -> Result<(), Box<dyn std::error::Error>> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/options/out/debug/options-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/options/out/debug/options.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();
    let expected_u64 = Some(36);
    let response = contract_instance
        .input_primitive(expected_u64)
        .call()
        .await?;

    assert!(response.value);

    let expected_struct = Some(s);
    let response = contract_instance
        .input_struct(expected_struct)
        .call()
        .await?;

    assert!(response.value);

    let expected_enum = Some(e);
    let response = contract_instance.input_enum(expected_enum).call().await?;

    assert!(response.value);

    let expected_none = None;
    let response = contract_instance.input_none(expected_none).call().await?;

    assert!(response.value);

    Ok(())
}

#[tokio::test]
async fn test_rust_result_can_be_decoded() -> Result<(), Box<dyn std::error::Error>> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/results/out/debug/results-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/results/out/debug/results.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let expected_ok_address = Ok(expected_address);
    let response = contract_instance.get_ok_address().call().await?;

    assert_eq!(response.value, expected_ok_address);

    let expected_some_u64 = Ok(10);
    let response = contract_instance.get_ok_u64().call().await?;

    assert_eq!(response.value, expected_some_u64);

    let response = contract_instance.get_ok_struct().call().await?;
    assert_eq!(response.value, Ok(s.clone()));

    let response = contract_instance.get_ok_enum().call().await?;
    assert_eq!(response.value, Ok(e.clone()));

    let response = contract_instance.get_ok_tuple().call().await?;
    assert_eq!(response.value, Ok((s, e)));

    let expected_error = Err(TestError::NoAddress("error".try_into().unwrap()));
    let response = contract_instance.get_error().call().await?;

    assert_eq!(response.value, expected_error);

    Ok(())
}

#[tokio::test]
async fn test_rust_result_can_be_encoded() -> Result<(), Box<dyn std::error::Error>> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/results/out/debug/results-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/results/out/debug/results.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let expected_ok_address = Ok(expected_address);
    let response = contract_instance
        .input_ok(expected_ok_address)
        .call()
        .await?;

    assert!(response.value);

    let expected_error = Err(TestError::NoAddress("error".try_into().unwrap()));
    let response = contract_instance.input_error(expected_error).call().await?;

    assert!(response.value);

    Ok(())
}
