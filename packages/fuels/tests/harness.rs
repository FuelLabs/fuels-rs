use fuels::prelude::{abigen, launch_provider_and_get_wallet, Error};
use fuels_core::Token;
use fuels_core::Tokenizable;
use sha2::{Digest, Sha256};

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
        "packages/fuels/tests/takes_ints_returns_bool.json",
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

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

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

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

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

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

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

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

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

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContractBuilder::new(null_contract_id(), wallet).build();

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
    assert!(nested_struct_from_tokens.y.a);

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
