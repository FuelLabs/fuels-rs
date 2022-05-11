use fuel_tx::{AssetId, ContractId, Receipt};
use fuels::prelude::Error;
use fuels::prelude::{
    setup_address_and_coins, setup_test_provider, setup_test_provider_and_wallet, CallParameters,
    Contract, LocalWallet, Provider, Signer, TxParameters, DEFAULT_INITIAL_BALANCE,
};
use fuels_abigen_macro::abigen;
use fuels_core::constants::NATIVE_ASSET_ID;
use fuels_core::Token;
use sha2::{Digest, Sha256};

/// Note: all the tests and examples below require pre-compiled Sway projects.
/// To compile these projects, run `cargo run --bin build-test-projects`.
/// It will build all test projects, creating their respective binaries,
/// ABI files, and lock files. These are not to be committed to the repository.

fn null_contract_id() -> String {
    // a null contract address ~[0u8;32]
    String::from("0000000000000000000000000000000000000000000000000000000000000000")
}

#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        "packages/fuels-abigen-macro/tests/takes_ints_returns_bool.json",
    );

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

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

    assert_eq!("000000009593586c000000000000002a", encoded);
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    //`SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_ints_returns_bool(42_u32);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("000000009593586c000000000000002a", encoded);
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
                        "name":"arg",
                        "type":"[u16; 3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let input: Vec<u16> = vec![1, 2, 3, 4];
    let contract_call = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "000000005898d3a40000000000000001000000000000000200000000000000030000000000000004",
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
                        "name":"arg",
                        "type":"[bool; 3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[

                ]
            }
        ]
        "#,
    );

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let input: Vec<bool> = vec![true, false, true];
    let contract_call = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "000000006fc82450000000000000000100000000000000000000000000000001",
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_byte(10 as u8);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
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
        [
            {
                "type":"contract",
                "inputs":[
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_string("This is a full sentence".into());

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
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
        [
            {
                "type":"contract",
                "inputs":[
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"value",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "[u8; 2]"
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
    // Because of the abigen! macro, `MyStruct` is now in scope
    // and can be used!
    let input = MyStruct {
        foo: vec![10, 2],
        bar: "fuel".to_string(),
    };

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!(
        "00000000ef5aac44000000000000000a00000000000000026675656c00000000",
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_nested_struct(input);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
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
        [
            {
                "type":"contract",
                "inputs":[
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_enum(variant);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("00000000082e0dfa0000000000000000000000000000002a", encoded);
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_struct(struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("00000000cb0b2f05000000000000000a0000000000000001", encoded);
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

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), provider, wallet);

    let contract_call = contract_instance.takes_nested_struct(nested_struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(contract_call.encoded_selector),
        hex::encode(contract_call.encoded_args)
    );

    assert_eq!("0000000088bf8a1b000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn example_workflow() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(
        &compiled,
        &provider.clone(),
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
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
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_output_test/out/debug/contract_output_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

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

    let response = contract_instance
        .return_my_struct(my_struct)
        .call()
        .await
        .unwrap();

    assert_eq!(response.value.foo, 10);
    assert!(response.value.bar);
}

#[tokio::test]
async fn call_with_structs() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/complex_types_contract/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/complex_types_contract/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);
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
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/call_empty_return/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/call_empty_return/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    let _result = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();
}

#[tokio::test]
async fn abigen_different_structs_same_arg_name() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/two_structs/out/debug/two_structs-abi.json",
    );

    let compiled =
        Contract::load_sway_contract("tests/test_projects/two_structs/out/debug/two_structs.bin")
            .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

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
async fn test_reverting_transaction() {
    abigen!(
        RevertingContract,
        "packages/fuels-abigen-macro/tests/test_projects/revert_transaction_error/out/debug/capture_revert_transaction_error-abi.json"
    );

    let compiled =
    Contract::load_sway_contract("tests/test_projects/revert_transaction_error/out/debug/capture_revert_transaction_error.bin")
        .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    let contract_instance = RevertingContract::new(contract_id.to_string(), provider, wallet);
    println!("Contract deployed @ {:x}", contract_id);
    let result = contract_instance.make_transaction_fail(0).call().await;
    assert!(matches!(result, Err(Error::ContractCallError(_))));
}

#[tokio::test]
async fn multiple_read_calls() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/multiple_read_calls/out/debug/demo-abi.json"
    );

    let compiled =
        Contract::load_sway_contract("tests/test_projects/multiple_read_calls/out/debug/demo.bin")
            .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    contract_instance.store(42).call().await.unwrap();

    // Use "simulate" because the methods don't actually run a transaction, but just a dry-run
    // We can notice here that, thanks to this, we don't generate a TransactionId collision,
    // even if the transactions are theoretically the same.
    let stored = contract_instance.read(0).simulate().await.unwrap();

    assert_eq!(stored.value, 42);

    let stored = contract_instance.read(0).simulate().await.unwrap();

    assert_eq!(stored.value, 42);
}

#[tokio::test]
async fn test_methods_typeless_argument() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/empty_arguments/out/debug/method_four_arguments-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/empty_arguments/out/debug/method_four_arguments.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    let result = contract_instance
        .method_with_empty_argument()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 63);
}

#[tokio::test]
async fn test_connect_to_deployed_contract() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let deployed_contract_instance =
        MyContract::new(contract_id.to_string(), provider.clone(), wallet.clone());

    // Check that the deployed contract works as expected.
    let result = deployed_contract_instance
        .initialize_counter(21)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 21);

    let result = deployed_contract_instance
        .increment_counter(21)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 42);

    let result = deployed_contract_instance
        .get_counter()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 42);

    // Create a new contract instance by connecting to
    // the previously deployed contract.
    let connected_contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    // Check that it works as expected.
    let result = connected_contract_instance
        .initialize_counter(111)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 111);

    let result = connected_contract_instance
        .increment_counter(9)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 120);

    let result = connected_contract_instance
        .get_counter()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 120);

    // Check that the originally deployed contract is in the same state.
    let result = deployed_contract_instance
        .get_counter()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 120);
}

#[tokio::test]
async fn test_large_return_data() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/large_return_data/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/large_return_data/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    let res = contract_instance.get_id().call().await.unwrap();

    assert_eq!(
        res.value,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );

    // One word-sized string
    let res = contract_instance.get_small_string().call().await.unwrap();
    assert_eq!(res.value, "gggggggg");

    // Two word-sized string
    let res = contract_instance.get_large_string().call().await.unwrap();
    assert_eq!(res.value, "ggggggggg");

    // Large struct will be bigger than a `WORD`.
    let res = contract_instance.get_large_struct().call().await.unwrap();
    assert_eq!(res.value.foo, 12);
    assert_eq!(res.value.bar, 42);

    // Array will be returned in `ReturnData`.
    let res = contract_instance.get_large_array().call().await.unwrap();
    assert_eq!(res.value, &[1, 2]);

    let res = contract_instance.get_contract_id().call().await.unwrap();

    // First `value` is from `CallResponse`.
    // Second `value` is from Sway `ContractId` type.
    assert_eq!(
        res.value,
        ContractId::from([
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ])
    );
}

#[tokio::test]
#[ignore]
async fn test_provider_launch_and_connect() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
    )
    .unwrap();

    let (pk, coins) = setup_address_and_coins(10, 10);
    let (launched_provider, address) = setup_test_provider(coins).await;
    let connected_provider = Provider::connect(address).await.unwrap();

    let wallet = LocalWallet::new_from_private_key(pk, launched_provider.clone());
    let contract_id = Contract::deploy(
        &compiled,
        &connected_provider,
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance_connected =
        MyContract::new(contract_id.to_string(), connected_provider, wallet.clone());

    let contract_instance_launched =
        MyContract::new(contract_id.to_string(), launched_provider, wallet);

    let result = contract_instance_connected
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();
    assert_eq!(42, result.value);

    let result = contract_instance_launched
        .increment_counter(10)
        .call()
        .await
        .unwrap();
    assert_eq!(52, result.value);
}

#[tokio::test]
async fn test_contract_calling_contract() {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    abigen!(
        FooContract,
        "packages/fuels-abigen-macro/tests/test_projects/foo_contract/out/debug/foo_contract-abi.json"
    );

    abigen!(
        FooCaller,
        "packages/fuels-abigen-macro/tests/test_projects/foo_caller_contract/out/debug/foo_caller_contract-abi.json"
    );

    // Load the first compiled contract
    let compiled =
        Contract::load_sway_contract("tests/test_projects/foo_contract/out/debug/foo_contract.bin")
            .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    let foo_contract_id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    println!("Foo contract deployed @ {:x}", foo_contract_id);

    let foo_contract_instance = FooContract::new(
        foo_contract_id.to_string(),
        provider.clone(),
        wallet.clone(),
    );

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.foo(true).call().await.unwrap();
    assert!(!res.value);

    // Compile and deploy second contract
    let compiled = Contract::load_sway_contract(
        "tests/test_projects/foo_caller_contract/out/debug/foo_caller_contract.bin",
    )
    .unwrap();

    let foo_caller_contract_id =
        Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
            .await
            .unwrap();
    println!(
        "Foo caller contract deployed @ {:x}",
        foo_caller_contract_id
    );

    let foo_caller_contract_instance = FooCaller::new(
        foo_caller_contract_id.to_string(),
        provider.clone(),
        wallet.clone(),
    );

    // Calls the contract that calls the `FooContract` contract, also just
    // flips the bool value passed to it.
    let res = foo_caller_contract_instance
        .call_foo_contract(*foo_contract_id, true)
        .set_contracts(&[foo_contract_id]) // Sets the external contract
        .call()
        .await
        .unwrap();

    assert!(!res.value);
}

#[tokio::test]
async fn test_gas_errors() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let contract_id = Contract::deploy(
        &compiled,
        &provider.clone(),
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let contract_instance = MyContract::new(contract_id.to_string(), provider, wallet);

    // Test for insufficient gas.
    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(
            Some(DEFAULT_INITIAL_BALANCE),
            Some(100),
            None,
            None,
        ))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    assert_eq!("Contract call error: Response errors; unexpected block execution error InsufficientFeeAmount { provided: 1000000000, required: 100000000000 }", result.to_string());

    // Test for running out of gas. Gas price as `None` will be 0.
    // Gas limit will be 100, this call will use more than 100 gas.
    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(100), None, None))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    assert_eq!("Contract call error: OutOfGas", result.to_string());
}

#[tokio::test]
async fn test_amount_and_asset_forwarding() {
    abigen!(
        TestFuelCoinContract,
        "packages/fuels-abigen-macro/tests/test_projects/token_ops/out/debug/token_ops-abi.json"
    );

    let compiled =
        Contract::load_sway_contract("tests/test_projects/token_ops/out/debug/token_ops.bin")
            .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = TestFuelCoinContract::new(id.to_string(), provider.clone(), wallet.clone());

    let mut balance_result = instance.get_balance(id, id).call().await.unwrap();
    assert_eq!(balance_result.value, 0);

    instance.mint_coins(5_000_000).call().await.unwrap();

    balance_result = instance.get_balance(id, id).call().await.unwrap();
    assert_eq!(balance_result.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);
    // Forward 1_000_000 coin amount of native asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &NATIVE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    instance
        .transfer_coins_to_output(1_000_000, id, address)
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let call_params = CallParameters::new(Some(0), Some(AssetId::from(*id)));
    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*id)
    );
}

#[tokio::test]
async fn test_multiple_args() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
    )
    .unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = MyContract::new(id.to_string(), provider.clone(), wallet.clone());

    // Make sure we can call the contract with multiple arguments
    let response = instance.get(5, 6).call().await.unwrap();

    assert_eq!(response.value, 5);

    let t = MyType { x: 5, y: 6 };
    let response = instance.get_alt(t).call().await.unwrap();
    assert_eq!(response.value, 5);

    let response = instance.get_single(5).call().await.unwrap();
    assert_eq!(response.value, 5);
}

#[tokio::test]
async fn test_tuples() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/tuples/out/debug/tuples-abi.json"
    );

    let compiled =
        Contract::load_sway_contract("tests/test_projects/tuples/out/debug/tuples.bin").unwrap();

    let (provider, wallet) = setup_test_provider_and_wallet().await;

    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = MyContract::new(id.to_string(), provider.clone(), wallet.clone());

    let response = instance.returns_tuple((1, 2)).call().await.unwrap();
    assert_eq!(response.value, (1, 2));
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() {
    abigen!(
        AuthContract,
        "packages/fuels-abigen-macro/tests/test_projects/auth_testing_contract/out/debug/auth_testing_contract-abi.json"
    );

    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let compiled = Contract::load_sway_contract(
        "tests/test_projects/auth_testing_contract/out/debug/auth_testing_contract.bin",
    )
    .unwrap();

    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();

    let auth_instance = AuthContract::new(id.to_string(), provider.clone(), wallet.clone());

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let result = auth_instance
        .check_msg_sender(wallet.address())
        .call()
        .await
        .unwrap();

    assert!(result.value);
}

#[tokio::test]
async fn workflow_enum_inside_struct() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/enum_inside_struct/out/debug\
        /enum_inside_struct-abi.json"
    );
    let compiled = Contract::load_sway_contract(
        "tests/test_projects/enum_inside_struct/out/debug/enum_inside_struct.bin",
    )
    .unwrap();
    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    let instance = MyContract::new(id.to_string(), provider.clone(), wallet.clone());
    let result = instance.return_enum_inside_struct(11).call().await.unwrap();
    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(222),
        glass: 333,
    };
    assert_eq!(result.value, expected);
}

#[tokio::test]
async fn workflow_struct_inside_enum() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/struct_inside_enum/out/debug/struct_inside_enum-abi.json"
    );

    let compiled = Contract::load_sway_contract(
        "tests/test_projects/struct_inside_enum/out/debug/struct_inside_enum.bin",
    )
    .unwrap();
    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = MyContract::new(id.to_string(), provider.clone(), wallet.clone());
    let result = instance.return_struct_inside_enum(11).call().await.unwrap();
    let expected = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 99 });
    assert_eq!(result.value, expected);
}

#[tokio::test]
async fn test_logd_receipts() {
    abigen!(
        LoggingContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_logdata/out/debug/contract_logdata-abi.json"
    );
    let (provider, wallet) = setup_test_provider_and_wallet().await;
    let compiled = Contract::load_sway_contract(
        "tests/test_projects/contract_logdata/out/debug/contract_logdata.bin",
    )
    .unwrap();

    let id = Contract::deploy(&compiled, &provider, &wallet, TxParameters::default())
        .await
        .unwrap();
    let contract_instance = LoggingContract::new(id.to_string(), provider.clone(), wallet.clone());
    let mut value = [0u8; 32];
    value[0] = 0xFF;
    value[1] = 0xEE;
    value[2] = 0xDD;
    value[12] = 0xAA;
    value[13] = 0xBB;
    value[14] = 0xCC;
    let result = contract_instance
        .use_logd_opcode(value, 3, 6)
        .call()
        .await
        .unwrap();
    assert_eq!(result.logs.unwrap(), vec!["ffeedd", "ffeedd000000"]);
    let result = contract_instance
        .use_logd_opcode(value, 14, 15)
        .call()
        .await
        .unwrap();
    assert_eq!(
        result.logs.unwrap(),
        vec![
            "ffeedd000000000000000000aabb",
            "ffeedd000000000000000000aabbcc"
        ]
    );
    let result = contract_instance.dont_use_logd().call().await.unwrap();
    assert_eq!(result.logs, None);
}
