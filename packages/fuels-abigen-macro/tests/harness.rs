use fuel_gql_client::fuel_tx::{AssetId, ContractId, Receipt};
use fuels::prelude::{
    abigen, launch_provider_and_get_single_wallet, setup_multiple_assets_coins,
    setup_single_asset_coins, setup_test_provider, CallParameters, Contract, Error, LocalWallet,
    Provider, Signer, TxParameters, DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS,
};
use fuels_core::tx::Address;
use fuels_core::Tokenizable;
use fuels_core::{constants::BASE_ASSET_ID, Token};
use sha2::{Digest, Sha256};
use std::str::FromStr;
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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;
    //`SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input: Vec<u16> = vec![1, 2, 3, 4];
    let call_handler = contract_instance.takes_array(input);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!(
        "00000000101cbeb50000000000000001000000000000000200000000000000030000000000000004",
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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let input: Vec<bool> = vec![true, false, true];
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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.takes_byte(10u8);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.takes_string("This is a full sentence".into());

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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let mut hasher = Sha256::new();
    hasher.update("test string".as_bytes());

    let arg = hasher.finalize();

    let call_handler = contract_instance.takes_b256(arg.into());

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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

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
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"my_enum",
                        "type":"enum MyEnum",
                        "components": [
                            {
                                "name": "X",
                                "type": "u32"
                            },
                            {
                                "name": "Y",
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

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

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
    let struct_from_tokens = MyStruct::from_token(Token::Struct(vec![foo, bar])).unwrap();

    assert_eq!(10, struct_from_tokens.foo);
    assert!(struct_from_tokens.bar);

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.takes_struct(struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
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
    let inner_struct_token = Token::Struct(vec![a.clone()]);
    let inner_struct_from_tokens = InnerStruct::from_token(inner_struct_token.clone()).unwrap();
    assert!(inner_struct_from_tokens.a);

    // Creating the whole nested struct `MyNestedStruct`
    // from tokens.
    // `x` is the token for the field `x` in `MyNestedStruct`
    // `a` is the token for the field `a` in `InnerStruct`
    let x = Token::U16(10);

    let nested_struct_from_tokens =
        MyNestedStruct::from_token(Token::Struct(vec![x, inner_struct_token])).unwrap();

    assert_eq!(10, nested_struct_from_tokens.x);
    assert!(nested_struct_from_tokens.y.a);

    let wallet = launch_provider_and_get_single_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.takes_nested_struct(nested_struct_from_tokens);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("0000000088bf8a1b000000000000000a0000000000000001", encoded);
}

#[tokio::test]
async fn type_safe_output_values() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_output_test/out/debug/contract_output_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/complex_types_contract/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);
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

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/call_empty_return/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/two_structs/out/debug/two_structs.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy("tests/test_projects/revert_transaction_error/out/debug/capture_revert_transaction_error.bin", &wallet, TxParameters::default())
        .await
        .unwrap();
    let contract_instance = RevertingContract::new(contract_id.to_string(), wallet);
    println!("Contract deployed @ {:x}", contract_id);
    let result = contract_instance.make_transaction_fail(0).call().await;
    assert!(matches!(result, Err(Error::ContractCallError(..))));
}

#[tokio::test]
async fn multiple_read_calls() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/multiple_read_calls/out/debug/demo-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/multiple_read_calls/out/debug/demo.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

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

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/empty_arguments/out/debug/method_four_arguments.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let result = contract_instance
        .method_with_empty_argument()
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 63);
}

#[tokio::test]
async fn test_large_return_data() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/large_return_data/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/large_return_data/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

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
async fn test_provider_launch_and_connect() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = LocalWallet::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, None).await;
    let connected_provider = Provider::connect(address).await.unwrap();

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance_connected = MyContract::new(contract_id.to_string(), wallet.clone());

    let result = contract_instance_connected
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await
        .unwrap();
    assert_eq!(42, result.value);

    wallet.set_provider(launched_provider);
    let contract_instance_launched = MyContract::new(contract_id.to_string(), wallet);

    let result = contract_instance_launched
        .increment_counter(10)
        .call()
        .await
        .unwrap();
    assert_eq!(52, result.value);
}

#[tokio::test]
async fn test_contract_calling_contract() -> Result<(), Error> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    abigen!(
        FooContract,
        "packages/fuels-abigen-macro/tests/test_projects/foo_contract/out/debug/foo_contract-abi.json"
    );

    abigen!(
        FooCaller,
        "packages/fuels-abigen-macro/tests/test_projects/foo_caller_contract/out/debug/foo_caller_contract-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    // Load and deploy the first compiled contract
    let foo_contract_id = Contract::deploy(
        "tests/test_projects/foo_contract/out/debug/foo_contract.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!("Foo contract deployed @ {:x}", foo_contract_id);

    let foo_contract_instance = FooContract::new(foo_contract_id.to_string(), wallet.clone());

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.foo(true).call().await.unwrap();
    assert!(!res.value);

    // Load and deploy the second compiled contract
    let foo_caller_contract_id = Contract::deploy(
        "tests/test_projects/foo_caller_contract/out/debug/foo_caller_contract.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    println!(
        "Foo caller contract deployed @ {:x}",
        foo_caller_contract_id
    );

    let foo_caller_contract_instance =
        FooCaller::new(foo_caller_contract_id.to_string(), wallet.clone());

    // Calls the contract that calls the `FooContract` contract, also just
    // flips the bool value passed to it.
    // ANCHOR: external_contract
    let res = foo_caller_contract_instance
        .call_foo_contract(*foo_contract_id, true)
        .set_contracts(&[foo_contract_id]) // Sets the external contract
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert!(!res.value);
    Ok(())
}

#[tokio::test]
async fn test_gas_errors() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    // Test for insufficient gas.
    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(
            Some(DEFAULT_COIN_AMOUNT),
            Some(100),
            None,
            None,
        ))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Contract call error: Response errors; unexpected block execution error \
    InsufficientFeeAmount { provided: 1000000000, required: 100000000000 }, receipts:";
    assert!(result.to_string().starts_with(expected));

    // Test for running out of gas. Gas price as `None` will be 0.
    // Gas limit will be 100, this call will use more than 100 gas.
    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(100), None, None))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(result.to_string().starts_with(expected));
}

#[tokio::test]
async fn test_call_param_gas_errors() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let result = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1000), None, None))
        .call_params(CallParameters::new(None, None, Some(1)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(result.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let result = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1), None, None))
        .call_params(CallParameters::new(None, None, Some(1000)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(result.to_string().starts_with(expected));
}

#[tokio::test]
async fn test_amount_and_asset_forwarding() {
    abigen!(
        TestFuelCoinContract,
        "packages/fuels-abigen-macro/tests/test_projects/token_ops/out/debug/token_ops-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/token_ops/out/debug/token_ops.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = TestFuelCoinContract::new(id.to_string(), wallet.clone());

    let mut balance_result = instance.get_balance(id, id).call().await.unwrap();
    assert_eq!(balance_result.value, 0);

    instance.mint_coins(5_000_000).call().await.unwrap();

    balance_result = instance.get_balance(id, id).call().await.unwrap();
    assert_eq!(balance_result.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

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
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    instance
        .transfer_coins_to_output(1_000_000, id, address)
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let call_params = CallParameters::new(Some(0), Some(AssetId::from(*id)), None);
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

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = MyContract::new(id.to_string(), wallet.clone());

    // Make sure we can call the contract with multiple arguments
    let response = instance.get(5, 6).call().await.unwrap();

    assert_eq!(response.value, 5);

    let t = MyType { x: 5, y: 6 };
    let response = instance.get_alt(t.clone()).call().await.unwrap();
    assert_eq!(response.value, t);

    let response = instance.get_single(5).call().await.unwrap();
    assert_eq!(response.value, 5);
}

#[tokio::test]
async fn test_tuples() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/tuples/out/debug/tuples-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/tuples/out/debug/tuples.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = MyContract::new(id.to_string(), wallet.clone());

    let response = instance.returns_tuple((1, 2)).call().await.unwrap();

    assert_eq!(response.value, (1, 2));

    // Tuple with struct.
    let my_struct_tuple = (
        42,
        Person {
            name: "Jane".to_string(),
        },
    );
    let response = instance
        .returns_struct_in_tuple(my_struct_tuple.clone())
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, my_struct_tuple);

    // Tuple with enum.
    let my_enum_tuple: (u64, State) = (42, State::A());

    let response = instance
        .returns_enum_in_tuple(my_enum_tuple.clone())
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, my_enum_tuple);

    let id = *ContractId::zeroed();
    let my_b256_u8_tuple: ([u8; 32], u8) = (id, 10);

    let response = instance
        .tuple_with_b256(my_b256_u8_tuple)
        .call()
        .await
        .unwrap();

    assert_eq!(response.value, my_b256_u8_tuple);
}

#[tokio::test]
async fn test_array() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    assert_eq!(
        contract_instance
            .get_array([42; 2].to_vec())
            .call()
            .await
            .unwrap()
            .value,
        [42; 2]
    );
}

#[tokio::test]
async fn test_arrays_with_custom_types() {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let persons = vec![
        Person {
            name: "John".to_string(),
        },
        Person {
            name: "Jane".to_string(),
        },
    ];

    let result = contract_instance
        .array_of_structs(persons)
        .call()
        .await
        .unwrap();

    assert_eq!("John", result.value[0].name);
    assert_eq!("Jane", result.value[1].name);

    let states = vec![State::A(), State::B()];

    let result = contract_instance
        .array_of_enums(states.clone())
        .call()
        .await
        .unwrap();

    assert_eq!(states[0], result.value[0]);
    assert_eq!(states[1], result.value[1]);
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() {
    abigen!(
        AuthContract,
        "packages/fuels-abigen-macro/tests/test_projects/auth_testing_contract/out/debug/auth_testing_contract-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/auth_testing_contract/out/debug/auth_testing_contract.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let auth_instance = AuthContract::new(id.to_string(), wallet.clone());

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

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/enum_inside_struct/out/debug/enum_inside_struct.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    let instance = MyContract::new(id.to_string(), wallet.clone());
    let result = instance.return_enum_inside_struct(11).call().await.unwrap();
    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(222),
        glass: 333,
    };
    assert_eq!(result.value, expected);
    let enum_inside_struct = Cocktail {
        the_thing_you_mix_in: Shaker::Cosmopolitan(444),
        glass: 555,
    };
    let result = instance
        .take_enum_inside_struct(enum_inside_struct)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 6666)
}

#[tokio::test]
async fn workflow_struct_inside_enum() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/struct_inside_enum/out/debug/struct_inside_enum-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/struct_inside_enum/out/debug/struct_inside_enum.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = MyContract::new(id.to_string(), wallet.clone());
    let result = instance.return_struct_inside_enum(11).call().await.unwrap();
    let expected = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 99 });
    assert_eq!(result.value, expected);
    let struct_inside_enum = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 66 });
    let result = instance
        .take_struct_inside_enum(struct_inside_enum)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 8888);
}

#[tokio::test]
async fn test_logd_receipts() {
    abigen!(
        LoggingContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_logdata/out/debug/contract_logdata-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/contract_logdata/out/debug/contract_logdata.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
    let contract_instance = LoggingContract::new(id.to_string(), wallet.clone());
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
    assert_eq!(result.logs, vec!["ffeedd", "ffeedd000000"]);
    let result = contract_instance
        .use_logd_opcode(value, 14, 15)
        .call()
        .await
        .unwrap();
    assert_eq!(
        result.logs,
        vec![
            "ffeedd000000000000000000aabb",
            "ffeedd000000000000000000aabbcc"
        ]
    );
    let result = contract_instance.dont_use_logd().call().await.unwrap();
    assert!(result.logs.is_empty());
}

#[tokio::test]
async fn test_wallet_balance_api() {
    // Single asset
    let mut wallet = LocalWallet::new_random(None);
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), None).await;
    wallet.set_provider(provider);
    for (_utxo_id, coin) in coins {
        let balance = wallet.get_asset_balance(&coin.asset_id).await;
        assert_eq!(balance.unwrap(), number_of_coins * amount_per_coin);
    }
    let balances = wallet.get_balances().await.unwrap();
    let expected_key = "0x".to_owned() + BASE_ASSET_ID.to_string().as_str();
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&expected_key));
    assert_eq!(
        *balances.get(&expected_key).unwrap(),
        number_of_coins * amount_per_coin
    );

    // Multiple assets
    let number_of_assets = 7;
    let coins_per_asset = 21;
    let amount_per_coin = 11;
    let (coins, asset_ids) = setup_multiple_assets_coins(
        wallet.address(),
        number_of_assets,
        coins_per_asset,
        amount_per_coin,
    );
    assert_eq!(coins.len() as u64, number_of_assets * coins_per_asset);
    assert_eq!(asset_ids.len() as u64, number_of_assets);
    let (provider, _) = setup_test_provider(coins.clone(), None).await;
    wallet.set_provider(provider);
    let balances = wallet.get_balances().await.unwrap();
    assert_eq!(balances.len() as u64, number_of_assets);
    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance.unwrap(), coins_per_asset * amount_per_coin);
        let expected_key = "0x".to_owned() + asset_id.to_string().as_str();
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            coins_per_asset * amount_per_coin
        );
    }
}

#[tokio::test]
async fn sway_native_types_support() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/sway_native_types/out/debug/sway_native_types-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/sway_native_types/out/debug/sway_native_types.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = MyContract::new(id.to_string(), wallet.clone());

    let user = User {
        weight: 10,
        address: Address::zeroed(),
    };
    let result = instance.wrapped_address(user).call().await.unwrap();

    assert_eq!(result.value.address, Address::zeroed());

    let result = instance
        .unwrapped_address(Address::zeroed())
        .call()
        .await
        .unwrap();

    assert_eq!(
        result.value,
        Address::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap()
    );
}

#[tokio::test]
async fn test_transaction_script_workflow() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;
    let client = &wallet.get_provider().unwrap().client;

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

    let call_handler = contract_instance.initialize_counter(42);

    let script = call_handler.get_script().await;
    assert!(script.tx.is_script());

    let receipts = script.call(client).await.unwrap();

    let response = call_handler.get_response(receipts).unwrap();
    assert_eq!(response.value, 42);
}

#[tokio::test]
async fn enums_are_correctly_encoded_and_decoded() {
    abigen!(
        EnumTesting,
        "packages/fuels-abigen-macro/tests/test_projects/enum_encoding/out/debug\
        /enum_encoding-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/enum_encoding/out/debug/enum_encoding.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = EnumTesting::new(id.to_string(), wallet);

    // If we had a regression on the issue of enum encoding width, then we'll
    // probably end up mangling arg_2 and onward which will fail this test.
    let expected = Bundle {
        arg_1: EnumThatHasABigAndSmallVariant::Small(12345),
        arg_2: 6666,
        arg_3: 7777,
        arg_4: 8888,
    };
    let actual = instance.get_bundle().call().await.unwrap().value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_bundle_integrity(expected)
        .call()
        .await
        .unwrap()
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the bundle correctly. Investigate!"
    );
}

#[tokio::test]
async fn enum_as_input() {
    abigen!(
        EnumTesting,
        "packages/fuels-abigen-macro/tests/test_projects/enum_as_input/out/debug\
        /enum_as_input-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/enum_as_input/out/debug/enum_as_input.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = EnumTesting::new(id.to_string(), wallet);

    let expected = StandardEnum::Two(12345);
    let actual = instance.get_standard_enum().call().await.unwrap().value;
    assert_eq!(expected, actual);

    let fuelvm_judgement = instance
        .check_standard_enum_integrity(expected)
        .call()
        .await
        .unwrap()
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the standard enum correctly. Investigate!"
    );

    let expected = UnitEnum::Two();
    let actual = instance.get_unit_enum().call().await.unwrap().value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_unit_enum_integrity(expected)
        .call()
        .await
        .unwrap()
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the unit enum correctly. Investigate!"
    );
}

#[tokio::test]
async fn nested_structs() {
    abigen!(
        NestedStructs,
        "packages/fuels-abigen-macro/tests/test_projects/nested_structs/out/debug\
        /nested_structs-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/nested_structs/out/debug/nested_structs.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = NestedStructs::new(id.to_string(), wallet);

    let expected = AllStruct {
        some_struct: SomeStruct { par_1: 12345 },
    };

    let actual = instance.get_struct().call().await.unwrap().value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_struct_integrity(expected)
        .call()
        .await
        .unwrap()
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the argument correctly. Investigate!"
    );
}

#[tokio::test]
async fn nested_enums_are_correctly_encoded_decoded() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/nested_enums/out/debug/nested_enums-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/nested_enums/out/debug/nested_enums.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    let instance = MyContract::new(id.to_string(), wallet.clone());

    let expected_enum = EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42)));

    let result = instance.get_nested_enum().call().await.unwrap();

    assert_eq!(result.value, expected_enum);

    let result = instance
        .check_nested_enum_integrity(expected_enum)
        .call()
        .await
        .unwrap();

    assert!(
        result.value,
        "The FuelVM deems that we've not encoded the nested enum correctly. Investigate!"
    );

    let expected_some_address = Option::Some(Identity::Address(Address::zeroed()));

    let result = instance.get_some_address().call().await.unwrap();

    assert_eq!(result.value, expected_some_address);

    let expected_none = Option::None();

    let result = instance.get_none().call().await.unwrap();

    assert_eq!(result.value, expected_none);
}
