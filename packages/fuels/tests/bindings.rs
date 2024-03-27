#[cfg(not(feature = "experimental"))]
use std::slice;
use std::str::FromStr;

use fuels::prelude::*;
#[cfg(not(feature = "experimental"))]
use fuels::{
    core::{codec::ABIEncoder, traits::Tokenizable},
    types::{Bits256, EvmAddress},
};
#[cfg(not(feature = "experimental"))]
use sha2::{Digest, Sha256};

pub fn null_contract_id() -> Bech32ContractId {
    // a bech32 contract address that decodes to [0u8;32]
    Bech32ContractId::from_str("fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2")
        .unwrap()
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "SimpleContract",
            project = "packages/fuels/tests/bindings/simple_contract"
        )),
        Deploy(
            name = "simple_contract_instance",
            contract = "SimpleContract",
            wallet = "wallet"
        ),
    );

    let call_handler = simple_contract_instance
        .methods()
        .takes_int_returns_bool(42);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!("000000005f68ee3d000000000000002a", encoded);
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_from_inline_contract() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_ints_returns_bool(42_u32);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!("000000009593586c000000000000002a", encoded);
    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_array_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input = [1, 2, 3];
    let call_handler = contract_instance.methods().takes_array(input);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!(
        "00000000101cbeb5000000000000000100000000000000020000000000000003",
        encoded
    );

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_bool_array_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input = [true, false, true];
    let call_handler = contract_instance.methods().takes_array(input);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!("000000000c2282260100010000000000", encoded);

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_string_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    // ANCHOR: contract_takes_string
    let call_handler = contract_instance.methods().takes_string(
        "This is a full sentence"
            .try_into()
            .expect("failed to convert string into SizedAsciiString"),
    );
    // ANCHOR_END: contract_takes_string

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!(
        "00000000d56e76515468697320697320612066756c6c2073656e74656e636500",
        encoded
    );

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_b256_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    // ANCHOR: 256_arg
    let arg: [u8; 32] = hasher.finalize().into();

    let call_handler = contract_instance.methods().takes_b256(Bits256(arg));
    // ANCHOR_END: 256_arg

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!(
        "0000000054992852d5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b",
        encoded
    );

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_evm_address_input() -> Result<()> {
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
                "type": "struct std::vm::evm::evm_address::EvmAddress",
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
                "name": "takes_evm_address",
                "output": {
                  "name": "",
                  "type": 0,
                  "typeArguments": null
                }
              }
            ]
          }
        "#,
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    // ANCHOR: evm_address_arg
    let b256 = Bits256(hasher.finalize().into());
    let arg = EvmAddress::from(b256);

    let call_handler = contract_instance.methods().takes_evm_address(arg);
    // ANCHOR_END: evm_address_arg

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!(
        "000000006ef3f9a50000000000000000000000005b44e4cb4e2c2298f4ac457ba8f82743f31e930b",
        encoded
    );

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_struct_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));
    // Because of the abigen! macro, `MyStruct` is now in scope
    // and can be used!
    let input = MyStruct {
        foo: [10, 2],
        bar: "fuel".try_into().unwrap(),
    };

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_struct(input);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!("000000008d4ab9b00a020000000000006675656c00000000", encoded);

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_nested_struct_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let inner_struct = InnerStruct { a: true };

    let input = MyNestedStruct {
        x: 10,
        foo: inner_struct,
    };

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance
        .methods()
        .takes_nested_struct(input.clone());
    let encoded_args = ABIEncoder::default()
        .encode(slice::from_ref(&input.into_token()))
        .unwrap()
        .resolve(0);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    assert_eq!("0000000088bf8a1b000000000000000a0100000000000000", encoded);

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn compile_bindings_enum_input() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(Contract(
        name = "SimpleContract",
        abi = r#"
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
    ));

    let variant = MyEnum::X(42);

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_enum(variant);

    let encoded_args = call_handler.contract_call.encoded_args.unwrap().resolve(0);
    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(encoded_args)
    );

    let expected = "0000000021b2784f0000000000000000000000000000002a";
    assert_eq!(encoded, expected);

    Ok(())
}

#[cfg(not(feature = "experimental"))]
#[tokio::test]
async fn shared_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "ContractA",
                project = "packages/fuels/tests/bindings/sharing_types/contract_a"
            ),
            Contract(
                name = "ContractB",
                project = "packages/fuels/tests/bindings/sharing_types/contract_b"
            ),
        ),
        Deploy(
            name = "contract_a",
            contract = "ContractA",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_b",
            contract = "ContractB",
            wallet = "wallet"
        ),
    );

    {
        let methods = contract_a.methods();

        {
            let shared_struct_2 = SharedStruct2 {
                a: 11u32,
                b: SharedStruct1 { a: 12u32 },
            };
            let shared_enum = SharedEnum::a(10u64);
            let response = methods
                .uses_shared_type(shared_struct_2.clone(), shared_enum.clone())
                .call()
                .await?
                .value;

            assert_eq!(response, (shared_struct_2, shared_enum));
        }
        {
            let same_name_struct =
                abigen_bindings::contract_a_mod::StructSameNameButDifferentInternals { a: 13u32 };
            let same_name_enum =
                abigen_bindings::contract_a_mod::EnumSameNameButDifferentInternals::a(14u32);
            let response = methods
                .uses_types_that_share_only_names(same_name_struct.clone(), same_name_enum.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, (same_name_struct, same_name_enum));
        }
        {
            let arg = UniqueStructToContractA {
                a: SharedStruct2 {
                    a: 15u32,
                    b: SharedStruct1 { a: 5u8 },
                },
            };
            let response = methods
                .uses_shared_type_inside_owned_one(arg.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, arg);
        }
    }
    {
        let methods = contract_b.methods();

        {
            let shared_struct_2 = SharedStruct2 {
                a: 11u32,
                b: SharedStruct1 { a: 12u32 },
            };
            let shared_enum = SharedEnum::a(10u64);
            let response = methods
                .uses_shared_type(shared_struct_2.clone(), shared_enum.clone())
                .call()
                .await?
                .value;

            assert_eq!(response, (shared_struct_2, shared_enum));
        }
        {
            let same_name_struct =
                abigen_bindings::contract_b_mod::StructSameNameButDifferentInternals { a: [13u64] };
            let same_name_enum =
                abigen_bindings::contract_b_mod::EnumSameNameButDifferentInternals::a([14u64]);
            let response = methods
                .uses_types_that_share_only_names(same_name_struct.clone(), same_name_enum.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, (same_name_struct, same_name_enum));
        }
        {
            let arg = UniqueStructToContractB {
                a: SharedStruct2 {
                    a: 15u32,
                    b: SharedStruct1 { a: 5u8 },
                },
            };
            let response = methods
                .uses_shared_type_inside_owned_one(arg.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, arg);
        }
    }

    Ok(())
}

#[cfg(feature = "test-type-paths")]
#[tokio::test]
async fn type_paths_respected() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "ContractA",
            project = "packages/fuels/tests/bindings/type_paths"
        )),
        Deploy(
            name = "contract_a_instance",
            contract = "ContractA",
            wallet = "wallet"
        ),
    );

    {
        let contract_a_type =
            abigen_bindings::contract_a_mod::contract_a_types::VeryCommonNameStruct {
                another_field: 10u32,
            };

        let rtn = contract_a_instance
            .methods()
            .test_function(AWrapper {
                field: contract_a_type,
            })
            .call()
            .await?
            .value;

        let rtn_using_the_other_type =
            abigen_bindings::contract_a_mod::another_lib::VeryCommonNameStruct { field_a: 10u32 };
        assert_eq!(rtn, rtn_using_the_other_type);
    }

    Ok(())
}
