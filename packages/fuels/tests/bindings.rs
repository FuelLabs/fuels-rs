use fuels::core::abi_encoder::ABIEncoder;
use fuels::prelude::*;
use sha2::{Digest, Sha256};
use std::{slice, str::FromStr};

pub fn null_contract_id() -> Bech32ContractId {
    // a bech32 contract address that decodes to [0u8;32]
    Bech32ContractId::from_str("fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2")
        .unwrap()
}

#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        "packages/fuels/tests/bindings/takes_ints_returns_bool-abi.json",
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_ints_returns_bool(42);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_ints_returns_bool(42_u32);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input = [1, 2, 3];
    let call_handler = contract_instance.methods().takes_array(input);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input = [true, false, true];
    let call_handler = contract_instance.methods().takes_array(input);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_byte(Byte(10u8));

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    // ANCHOR: contract_takes_string
    let call_handler = contract_instance.methods().takes_string(
        "This is a full sentence"
            .try_into()
            .expect("failed to convert string into SizedAsciiString"),
    );
    // ANCHOR_END: contract_takes_string

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    // ANCHOR: 256_arg
    let arg: [u8; 32] = hasher.finalize().into();

    let call_handler = contract_instance.methods().takes_b256(Bits256(arg));
    // ANCHOR_END: 256_arg

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_struct(input);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance
        .methods()
        .takes_nested_struct(input.clone());
    let encoded_args = ABIEncoder::encode(slice::from_ref(&input.into_token()))
        .unwrap()
        .resolve(0);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
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

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_enum(variant);

    let encoded_args = call_handler.contract_call.encoded_args.resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(&encoded_args)
    );

    let expected = "0000000021b2784f0000000000000000000000000000002a";
    assert_eq!(encoded, expected);
}
