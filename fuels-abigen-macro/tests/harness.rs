use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;
use fuel_tx::Salt;
use fuels_abigen_macro::abigen;
use fuels_contract::contract::Contract;
use fuels_core::Token;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

async fn setup_local_node() -> FuelClient {
    let srv = FuelService::new_node(Config::local_node()).await.unwrap();
    FuelClient::from(srv.bound_address)
}

#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        "fuels-abigen-macro/tests/takes_ints_returns_bool.json",
    );

    let fuel_client = setup_local_node().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    // Calls the function defined in the JSON ABI.
    // Note that this is type-safe, if the function does exist
    // in the JSON ABI, this won't compile!
    // Currently this prints `0000000003b568d4000000000000002a000000000000000a`
    // The encoded contract call. Soon it'll be able to perform the
    // actual call.
    let contract_call = contract_instance.takes_ints_returns_bool(42);

    // Then you'll be able to use `.call()` to actually call the contract with the
    // specified function:
    // function.call().unwrap();
    // Or you might want to just `contract_instance.takes_u32_returns_bool(42 as u32).call()?`

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("00000000155799f1000000000000002a", encoded);
}

#[tokio::test]
async fn compile_bindings_from_inline_contract() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type": "function",
                "inputs": [
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name": "only_argument",
                        "type": "u32"
                    }
                ],
                "name": "takes_ints_returns_bool",
                "outputs": [
                    {
                        "name": "",
                        "type": "bool"
                    }
                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_ints_returns_bool(42 as u32);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("00000000155799f1000000000000002a", encoded);
}

#[tokio::test]
async fn compile_bindings_single_param() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type": "function",
                "inputs": [
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name": "arg",
                        "type": "u32"
                    }
                ],
                "name": "takes_ints_returns_bool",
                "outputs": [
                    {
                        "name": "ret",
                        "type": "bool"
                    }
                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_ints_returns_bool(42);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("00000000155799f1000000000000002a", encoded);
}

#[tokio::test]
async fn compile_bindings_array_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"arg",
                        "type":"u16[3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let input: Vec<u16> = vec![1, 2, 3, 4];
    let contract_call = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "00000000530300750000000000000001000000000000000200000000000000030000000000000004",
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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"arg",
                        "type":"bool[3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let input: Vec<bool> = vec![true, false, true];
    let contract_call = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "000000000abaed98000000000000000100000000000000000000000000000001",
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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"arg",
                        "type":"byte"
                    }
                ],
                "name":"takes_byte",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_byte(10 as u8);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("000000001be28a53000000000000000a", encoded);
}

#[tokio::test]
async fn compile_bindings_string_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"arg",
                        "type":"str[23]"
                    }
                ],
                "name":"takes_string",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_string("This is a full sentence".into());

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "00000000da2c7a675468697320697320612066756c6c2073656e74656e636500",
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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"arg",
                        "type":"b256"
                    }
                ],
                "name":"takes_b256",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    let arg = hasher.finalize();

    let contract_call = contract_instance.takes_b256(arg.into());

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "00000000250fb0f2d5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b",
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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"value",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8[2]"
                            },
                            {
                                "name": "bar",
                                "type": "str[4]"
                            }
                        ]
                    }
                ],
                "name":"takes_struct",
                "outputs":[]
            }
        ]
        "#,
    );

    let fuel_client = setup_local_node().await;

    // Because of the abigen! macro, `MyStruct` is now in scope
    // and can be used!
    let input = MyStruct {
        foo: vec![10, 2],
        bar: "fuel".to_string(),
    };

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "0000000081dae8d1000000000000000a00000000000000026675656c00000000",
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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"top_value",
                        "type":"struct MyNestedStruct",
                        "components": [
                            {
                                "name": "x",
                                "type": "u16"
                            },
                            {
                                "name": "foo",
                                "type": "struct InnerStruct",
                                "components": [
                                    {
                                        "name":"a",
                                        "type": "bool"
                                    }
                                ]
                            }
                        ]
                    }
                ],
                "name":"takes_nested_struct",
                "outputs":[]
            }
        ]
        "#,
    );

    let inner_struct = InnerStruct { a: true };

    let input = MyNestedStruct {
        x: 10,
        foo: inner_struct,
    };

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_nested_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("0000000074c481ed000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn compile_bindings_enum_input() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"my_enum",
                        "type":"enum MyEnum",
                        "components": [
                            {
                                "name": "x",
                                "type": "u32"
                            },
                            {
                                "name": "y",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_enum",
                "outputs":[]
            }
        ]
        "#,
    );

    let variant = MyEnum::X(42);

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_enum(variant);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("0000000085dab9fc0000000000000000000000000000002a", encoded);
}

#[tokio::test]
async fn create_struct_from_decoded_tokens() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"my_val",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_struct",
                "outputs":[]
            }
        ]
        "#,
    );

    // Decoded tokens
    let foo = Token::U8(10);
    let bar = Token::Bool(true);

    // Create the struct using the decoded tokens.
    // `struct_from_tokens` is of type `MyStruct`.
    let struct_from_tokens = MyStruct::new_from_tokens(&[foo, bar]);

    assert_eq!(10, struct_from_tokens.foo);
    assert!(struct_from_tokens.bar);

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_struct(struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("0000000087e388fd000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn create_nested_struct_from_decoded_tokens() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"input",
                        "type":"struct MyNestedStruct",
                        "components": [
                            {
                                "name": "x",
                                "type": "u16"
                            },
                            {
                                "name": "y",
                                "type": "struct InnerStruct",
                                "components": [
                                    {
                                        "name":"a",
                                        "type": "bool"
                                    }
                                ]
                            }
                        ]
                    }
                ],
                "name":"takes_nested_struct",
                "outputs":[]
            }
        ]
        "#,
    );

    // Creating just the InnerStruct is possible
    let a = Token::Bool(true);
    let inner_struct_from_tokens = InnerStruct::new_from_tokens(&[a.clone()]);
    assert!(inner_struct_from_tokens.a);

    // Creating the whole nested struct `MyNestedStruct`
    // from tokens.
    // `x` is the token for the field `x` in `MyNestedStruct`
    // `a` is the token for the field `a` in `InnerStruct`
    let x = Token::U16(10);

    let nested_struct_from_tokens = MyNestedStruct::new_from_tokens(&[x, a]);

    assert_eq!(10, nested_struct_from_tokens.x);
    assert!(nested_struct_from_tokens.y.a);

    let fuel_client = setup_local_node().await;

    let contract_instance = SimpleContract::new(Default::default(), fuel_client);

    let contract_call = contract_instance.takes_nested_struct(nested_struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("0000000074c481ed000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn example_workflow() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        r#"
        [
            {
                "inputs": [
                {
                    "components": null,
                    "name": "gas_",
                    "type": "u64"
                },
                {
                    "components": null,
                    "name": "amount_",
                    "type": "u64"
                },
                {
                    "components": null,
                    "name": "color_",
                    "type": "b256"
                },
                {
                    "components": null,
                    "name": "value",
                    "type": "u64"
                }
                ],
                "name": "initialize_counter",
                "outputs": [
                {
                    "components": null,
                    "name": "",
                    "type": "u64"
                }
                ],
                "type": "function"
            },
            {
                "inputs": [
                {
                    "components": null,
                    "name": "gas_",
                    "type": "u64"
                },
                {
                    "components": null,
                    "name": "amount_",
                    "type": "u64"
                },
                {
                    "components": null,
                    "name": "color_",
                    "type": "b256"
                },
                {
                    "components": null,
                    "name": "value",
                    "type": "u64"
                }
                ],
                "name": "increment_counter",
                "outputs": [
                {
                    "components": null,
                    "name": "",
                    "type": "u64"
                }
                ],
                "type": "function"
            }
        ]
        "#,
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/contract_test", salt).unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();

    assert_eq!(42, result.value);

    let result = contract_instance
        .increment_counter(10)
        .call()
        .await
        .unwrap();

    assert_eq!(52, result.value);
}

#[tokio::test]
async fn type_safe_output_values() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        MyContract,
        r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"value",
                        "type":"u64",
                        "components": []
                    }
                ],
                "name":"is_even",
                "outputs":[
                    {
                        "name":"ret",
                        "type":"bool",
                        "components": []
                    }
                ]
            },
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"value",
                        "type":"str[4]",
                        "components": []
                    }
                ],
                "name":"return_my_string",
                "outputs":[
                    {
                        "name":"ret",
                        "type":"str[4]",
                        "components": []
                    }
                ]
            },
            {
                "type":"contract",
                "inputs":[
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "name":"value",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"return_my_struct",
                "outputs":[
                    {
                        "name":"ret",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    }
                ]
            }
        ]
        "#,
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/contract_output_test", salt).unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    // `response`'s type matches the return type of `is_event()`
    let response = contract_instance.is_even(10).call().await.unwrap();
    assert!(response.value);

    // `response`'s type matches the return type of `return_my_string()`
    let response = contract_instance
        .return_my_string("fuel".to_string())
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, "fuel");

    let my_struct = MyStruct { foo: 10, bar: true };

    let _response = contract_instance.return_my_struct(my_struct).call().await;
}

#[tokio::test]
async fn call_with_structs() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        r#"
        [
            {
                "inputs": [
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "components": [
                            {
                                "components": null,
                                "name": "dummy",
                                "type": "bool"
                            },
                            {
                                "components": null,
                                "name": "initial_value",
                                "type": "u64"
                            }
                        ],
                        "name": "config",
                        "type": "struct CounterConfig"
                    }
                ],
                "name": "initialize_counter",
                "outputs": [
                    {
                        "components": null,
                        "name": "",
                        "type": "u64"
                    }
                ],
                "type": "function"
            },
            {
                "inputs": [
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "components": null,
                        "name": "amount",
                        "type": "u64"
                    }
                ],
                "name": "increment_counter",
                "outputs": [
                    {
                        "components": null,
                        "name": "",
                        "type": "u64"
                    }
                ],
                "type": "function"
            }
        ]
        "#,
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/complex_types_contract", salt)
            .unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    let counter_config = CounterConfig {
        dummy: true,
        initial_value: 42,
    };

    let result = contract_instance
        .initialize_counter(counter_config) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();

    assert_eq!(42, result.value);

    let result = contract_instance
        .increment_counter(10)
        .call()
        .await
        .unwrap();

    assert_eq!(52, result.value);
}

#[tokio::test]
async fn call_with_empty_return() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        r#"
        [
            {
                "inputs": [
                    {
                        "components": null,
                        "name": "gas_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount_",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color_",
                        "type": "b256"
                    },
                    {
                        "components": null,
                        "name": "val",
                        "type": "u64"
                    }
                ],
                "name": "store_value",
                "outputs": [
                    {
                        "components": null,
                        "name": "",
                        "type": "()"
                    }
                ],
                "type": "function"
            }
        ]
        "#,
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/call_empty_return", salt).unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    let _result = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();
}

#[tokio::test]
async fn abigen_different_structs_same_arg_name() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    abigen!(
        MyContract,
        "fuels-abigen-macro/tests/test_projects/two-structs/abi.json",
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/two-structs", salt).unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    let param_one = StructOne { foo: 42 };
    let param_two = StructTwo { bar: 42 };

    let res_one = contract_instance.something(param_one).call().await.unwrap();

    assert_eq!(res_one.value, 43);

    let res_two = contract_instance
        .something_else(param_two)
        .call()
        .await
        .unwrap();

    assert_eq!(res_two.value, 41);
}

#[tokio::test]
async fn multiple_read_calls() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    abigen!(
        MyContract,
        "fuels-abigen-macro/tests/test_projects/multiple-read-calls/abi.json"
    );

    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/multiple-read-calls", salt).unwrap();

    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();

    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(compiled, client);

    contract_instance.store(42).call().await.unwrap();

    let stored = contract_instance.read(0).call().await.unwrap();

    assert!(stored.value == 42);

    let stored = contract_instance.read(0).call().await.unwrap();

    assert!(stored.value == 42);
}

#[tokio::test]
async fn test_methods_typeless_argument() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "fuels-abigen-macro/tests/test_projects/empty-arguments/abi.json"
    );
    // Build the contract
    let salt: [u8; 32] = rng.gen();
    let salt = Salt::from(salt);

    let compiled =
        Contract::compile_sway_contract("tests/test_projects/empty-arguments", salt).unwrap();
    let (client, contract_id) = Contract::launch_and_deploy(&compiled).await.unwrap();
    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(compiled, client);
    let result = contract_instance
        .method_with_empty_parenthesis_argument()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 21);
    let result = contract_instance
        .method_with_empty_string_argument()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 42);
    let result = contract_instance
        .method_with_empty_argument()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 63);
}
