use std::str::FromStr;

use fuels::{
    prelude::*,
    types::{Bits256, EvmAddress, Identity, SizedAsciiString, B512, U256},
};

pub fn null_contract_id() -> Bech32ContractId {
    // a bech32 contract address that decodes to [0u8;32]
    Bech32ContractId::from_str("fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2")
        .expect("is valid")
}

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/empty_arguments"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance
        .methods()
        .method_with_empty_argument()
        .call()
        .await?;

    assert_eq!(response.value, 63);

    Ok(())
}

#[tokio::test]
async fn call_with_empty_return() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/types/contracts/call_empty_return"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let _response = contract_instance.methods().store_value(42).call().await?;
    Ok(())
}

#[tokio::test]
async fn call_with_structs() -> Result<()> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    // ANCHOR: struct_generation
    abigen!(Contract(name="MyContract",
                     abi="e2e/sway/types/contracts/complex_types_contract/out/release/complex_types_contract-abi.json"));

    // Here we can use `CounterConfig`, a struct originally
    // defined in the contract.
    let counter_config = CounterConfig {
        dummy: true,
        initial_value: 42,
    };
    // ANCHOR_END: struct_generation

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_id = Contract::load_from(
        "sway/types/contracts/complex_types_contract/out/release/complex_types_contract.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet).methods();

    let response = contract_methods
        .initialize_counter(counter_config)
        .call()
        .await?;

    assert_eq!(42, response.value);

    let response = contract_methods.increment_counter(10).call().await?;

    assert_eq!(52, response.value);

    Ok(())
}

#[tokio::test]
async fn abigen_different_structs_same_arg_name() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/two_structs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let param_one = StructOne { foo: 42 };
    let param_two = StructTwo { bar: 42 };

    let contract_methods = contract_instance.methods();
    let res_one = contract_methods.something(param_one).call().await?;

    assert_eq!(res_one.value, 43);

    let res_two = contract_methods.something_else(param_two).call().await?;

    assert_eq!(res_two.value, 41);
    Ok(())
}

#[tokio::test]
async fn nested_structs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/nested_structs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let expected = AllStruct {
        some_struct: SomeStruct {
            field: 12345,
            field_2: true,
        },
    };

    let contract_methods = contract_instance.methods();
    let actual = contract_methods.get_struct().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_methods
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

    let actual = contract_methods
        .nested_struct_with_reserved_keyword_substring(call_data.clone())
        .call()
        .await?
        .value;

    assert_eq!(actual, call_data);
    Ok(())
}

#[tokio::test]
async fn calls_with_empty_struct() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/complex_types_contract"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let response = contract_methods.get_empty_struct().call().await?;

        assert_eq!(response.value, EmptyStruct {});
    }
    {
        let response = contract_methods
            .input_empty_struct(EmptyStruct {})
            .call()
            .await?;

        assert!(response.value);
    }

    Ok(())
}

#[tokio::test]
async fn can_use_try_into_to_construct_struct_from_bytes() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/types/contracts/enum_inside_struct/out/release\
        /enum_inside_struct-abi.json"
    ));
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
async fn test_tuples() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/tuples"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let response = contract_methods.returns_tuple((1, 2)).call().await?;

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
        let response = contract_methods
            .returns_struct_in_tuple(my_struct_tuple.clone())
            .call()
            .await?;

        assert_eq!(response.value, my_struct_tuple);
    }
    {
        // Tuple with enum.
        let my_enum_tuple: (u64, State) = (42, State::A);

        let response = contract_methods
            .returns_enum_in_tuple(my_enum_tuple.clone())
            .call()
            .await?;

        assert_eq!(response.value, my_enum_tuple);
    }
    {
        // Tuple with single element
        let my_enum_tuple = (123u64,);

        let response = contract_methods
            .single_element_tuple(my_enum_tuple)
            .call()
            .await?;

        assert_eq!(response.value, my_enum_tuple);
    }
    {
        // tuple with b256
        let id = *ContractId::zeroed();
        let my_b256_u8_tuple = (Bits256(id), 10);

        let response = contract_methods
            .tuple_with_b256(my_b256_u8_tuple)
            .call()
            .await?;

        assert_eq!(response.value, my_b256_u8_tuple);
    }

    Ok(())
}

#[tokio::test]
async fn test_evm_address() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/evm_address"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    {
        // ANCHOR: evm_address_arg
        let b256 = Bits256::from_hex_str(
            "0x1616060606060606060606060606060606060606060606060606060606060606",
        )?;
        let evm_address = EvmAddress::from(b256);

        let call_handler = contract_instance
            .methods()
            .evm_address_as_input(evm_address);
        // ANCHOR_END: evm_address_arg

        assert!(call_handler.call().await?.value);
    }

    {
        let b256 = Bits256::from_hex_str(
            "0x0606060606060606060606060606060606060606060606060606060606060606",
        )?;
        let expected_evm_address = EvmAddress::from(b256);

        assert_eq!(
            contract_instance
                .methods()
                .evm_address_from_literal()
                .call()
                .await?
                .value,
            expected_evm_address
        );
    }

    {
        let b256 = Bits256::from_hex_str(
            "0x0606060606060606060606060606060606060606060606060606060606060606",
        )?;
        let expected_evm_address = EvmAddress::from(b256);

        assert_eq!(
            contract_instance
                .methods()
                .evm_address_from_argument(b256)
                .call()
                .await?
                .value,
            expected_evm_address
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_array() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    assert_eq!(
        contract_instance
            .methods()
            .get_array([42; 2])
            .call()
            .await?
            .value,
        [42; 2]
    );
    Ok(())
}

#[tokio::test]
async fn test_arrays_with_custom_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let persons = [
        Person {
            name: "John".try_into()?,
        },
        Person {
            name: "Jane".try_into()?,
        },
    ];

    let contract_methods = contract_instance.methods();
    let response = contract_methods.array_of_structs(persons).call().await?;

    assert_eq!("John", response.value[0].name);
    assert_eq!("Jane", response.value[1].name);

    let states = [State::A, State::B];

    let response = contract_methods
        .array_of_enums(states.clone())
        .call()
        .await?;

    assert_eq!(states[0], response.value[0]);
    assert_eq!(states[1], response.value[1]);
    Ok(())
}

#[tokio::test]
async fn str_in_array() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/str_in_array"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let input = ["foo", "bar", "baz"].map(|str| str.try_into().unwrap());
    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .take_array_string_shuffle(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["baz", "foo", "bar"]);

    let response = contract_methods
        .take_array_string_return_single(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["foo"]);

    let response = contract_methods
        .take_array_string_return_single_element(input)
        .call()
        .await?;

    assert_eq!(response.value, "bar");
    Ok(())
}

#[tokio::test]
async fn test_enum_inside_struct() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/enum_inside_struct"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(11),
        glass: 333,
    };

    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .return_enum_inside_struct(11)
        .call()
        .await?;

    assert_eq!(response.value, expected);

    let enum_inside_struct = Cocktail {
        the_thing_you_mix_in: Shaker::Cosmopolitan(444),
        glass: 555,
    };

    let response = contract_methods
        .take_enum_inside_struct(enum_inside_struct)
        .call()
        .await?;

    assert_eq!(response.value, 555);
    Ok(())
}

#[tokio::test]
async fn native_types_support() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/native_types"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let user = User {
        weight: 10,
        address: Address::zeroed(),
    };

    let contract_methods = contract_instance.methods();
    let response = contract_methods.wrapped_address(user).call().await?;

    assert_eq!(response.value.address, Address::zeroed());

    let response = contract_methods
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
async fn enum_coding_w_variable_width_variants() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/enum_encoding"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    // If we had a regression on the issue of enum encoding width, then we'll
    // probably end up mangling arg_2 and onward which will fail this test.
    let expected = BigBundle {
        arg_1: EnumThatHasABigAndSmallVariant::Small(12345),
        arg_2: 6666,
        arg_3: 7777,
        arg_4: 8888,
    };

    let contract_methods = contract_instance.methods();
    let actual = contract_methods.get_big_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_methods
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
async fn enum_coding_w_unit_enums() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/enum_encoding"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    // If we had a regression on the issue of unit enum encoding width, then
    // we'll end up mangling arg_2
    let expected = UnitBundle {
        arg_1: UnitEnum::var2,
        arg_2: u64::MAX,
    };

    let contract_methods = contract_instance.methods();
    let actual = contract_methods.get_unit_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_methods
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
async fn enum_as_input() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/enum_as_input"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let expected = MaxedOutVariantsEnum::Variant255(11);
    let contract_methods = contract_instance.methods();
    let actual = contract_methods.get_max_variant().call().await?.value;
    assert_eq!(expected, actual);

    let expected = StandardEnum::Two(12345);
    let contract_methods = contract_instance.methods();
    let actual = contract_methods.get_standard_enum().call().await?.value;
    assert_eq!(expected, actual);

    let fuelvm_judgement = contract_methods
        .check_standard_enum_integrity(expected)
        .call()
        .await?
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the standard enum correctly. Investigate!"
    );

    let expected = UnitEnum::Two;
    let actual = contract_methods.get_unit_enum().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = contract_methods
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
async fn can_use_try_into_to_construct_enum_from_bytes() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/types/contracts/enum_inside_struct/out/release\
        /enum_inside_struct-abi.json"
    ));
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
    Ok(())
}

#[tokio::test]
async fn type_inside_enum() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/type_inside_enum"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    // String inside enum
    let enum_string = SomeEnum::SomeStr("asdf".try_into()?);
    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .str_inside_enum(enum_string.clone())
        .call()
        .await?;
    assert_eq!(response.value, enum_string);

    // Array inside enum
    let enum_array = SomeEnum::SomeArr([1, 2, 3, 4]);
    let response = contract_methods
        .arr_inside_enum(enum_array.clone())
        .call()
        .await?;
    assert_eq!(response.value, enum_array);

    // Struct inside enum
    let response = contract_methods
        .return_struct_inside_enum(11)
        .call()
        .await?;
    let expected = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 11 });
    assert_eq!(response.value, expected);

    let struct_inside_enum = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 66 });
    let response = contract_methods
        .take_struct_inside_enum(struct_inside_enum)
        .call()
        .await?;
    assert_eq!(response.value, 8888);

    // Enum inside enum
    let expected_enum = EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42)));
    let response = contract_methods.get_nested_enum().call().await?;
    assert_eq!(response.value, expected_enum);

    let response = contract_methods
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
async fn test_rust_option_can_be_decoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/options"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let expected_some_address = Some(expected_address);
    let response = contract_methods.get_some_address().call().await?;

    assert_eq!(response.value, expected_some_address);

    let expected_some_u64 = Some(10);
    let response = contract_methods.get_some_u64().call().await?;

    assert_eq!(response.value, expected_some_u64);

    let response = contract_methods.get_some_struct().call().await?;
    assert_eq!(response.value, Some(s.clone()));

    let response = contract_methods.get_some_enum().call().await?;
    assert_eq!(response.value, Some(e.clone()));

    let response = contract_methods.get_some_tuple().call().await?;
    assert_eq!(response.value, Some((s.clone(), e.clone())));

    let expected_none = None;
    let response = contract_methods.get_none().call().await?;

    assert_eq!(response.value, expected_none);

    Ok(())
}

#[tokio::test]
async fn test_rust_option_can_be_encoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/options"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let expected_u64 = Some(36);
    let response = contract_methods
        .input_primitive(expected_u64)
        .call()
        .await?;

    assert!(response.value);

    let expected_struct = Some(s);
    let response = contract_methods
        .input_struct(expected_struct)
        .call()
        .await?;

    assert!(response.value);

    let expected_enum = Some(e);
    let response = contract_methods.input_enum(expected_enum).call().await?;

    assert!(response.value);

    let expected_none = None;
    let response = contract_methods.input_none(expected_none).call().await?;

    assert!(response.value);

    Ok(())
}

#[tokio::test]
async fn test_rust_result_can_be_decoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/results"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        option: Some(expected_address),
    };

    let e = TestEnum::EnumOption(Some(expected_address));

    let expected_ok_address = Ok(expected_address);
    let response = contract_methods.get_ok_address().call().await?;

    assert_eq!(response.value, expected_ok_address);

    let expected_some_u64 = Ok(10);
    let response = contract_methods.get_ok_u64().call().await?;

    assert_eq!(response.value, expected_some_u64);

    let response = contract_methods.get_ok_struct().call().await?;
    assert_eq!(response.value, Ok(s.clone()));

    let response = contract_methods.get_ok_enum().call().await?;
    assert_eq!(response.value, Ok(e.clone()));

    let response = contract_methods.get_ok_tuple().call().await?;
    assert_eq!(response.value, Ok((s, e)));

    let expected_error = Err(TestError::NoAddress("error".try_into().unwrap()));
    let response = contract_methods.get_error().call().await?;

    assert_eq!(response.value, expected_error);

    Ok(())
}

#[tokio::test]
async fn test_rust_result_can_be_encoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/results"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let expected_ok_address = Ok(expected_address);
    let response = contract_methods
        .input_ok(expected_ok_address)
        .call()
        .await?;

    assert!(response.value);

    let expected_error = Err(TestError::NoAddress("error".try_into().unwrap()));
    let response = contract_methods.input_error(expected_error).call().await?;

    assert!(response.value);

    Ok(())
}

#[tokio::test]
async fn test_identity_can_be_decoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/identity"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;
    let expected_contract_id =
        ContractId::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        identity: Identity::Address(expected_address),
    };

    let e = TestEnum::EnumIdentity(Identity::ContractId(expected_contract_id));

    let response = contract_methods.get_identity_address().call().await?;
    assert_eq!(response.value, Identity::Address(expected_address));

    let response = contract_methods.get_identity_contract_id().call().await?;
    assert_eq!(response.value, Identity::ContractId(expected_contract_id));

    let response = contract_methods.get_struct_with_identity().call().await?;
    assert_eq!(response.value, s.clone());

    let response = contract_methods.get_enum_with_identity().call().await?;
    assert_eq!(response.value, e.clone());

    let response = contract_methods.get_identity_tuple().call().await?;
    assert_eq!(response.value, (s, e));

    Ok(())
}

#[tokio::test]
async fn test_identity_can_be_encoded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/identity"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;
    let expected_contract_id =
        ContractId::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    let s = TestStruct {
        identity: Identity::Address(expected_address),
    };

    let e = TestEnum::EnumIdentity(Identity::ContractId(expected_contract_id));

    let response = contract_methods
        .input_identity(Identity::Address(expected_address))
        .call()
        .await?;

    assert!(response.value);

    let response = contract_methods
        .input_struct_with_identity(s)
        .call()
        .await?;

    assert!(response.value);

    let response = contract_methods.input_enum_with_identity(e).call().await?;

    assert!(response.value);

    Ok(())
}

#[tokio::test]
async fn test_identity_with_two_contracts() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/identity"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_instance2",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    let expected_address =
        Address::from_str("0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0")?;

    {
        let response = contract_instance
            .methods()
            .input_identity(Identity::Address(expected_address))
            .call()
            .await?;

        assert!(response.value);
    }
    {
        let response = contract_instance2
            .methods()
            .input_identity(Identity::Address(expected_address))
            .call()
            .await?;

        assert!(response.value);
    }

    Ok(())
}

#[tokio::test]
async fn generics_test() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/generics"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        // ANCHOR: generic
        // simple struct with a single generic param
        let arg1 = SimpleGeneric {
            single_generic_param: 123u64,
        };

        let result = contract_methods
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

        let result = contract_methods
            .struct_delegating_generic(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // struct that has the generic in an array
        let arg1 = StructWArrayGeneric { a: [1u32, 2u32] };

        let result = contract_methods
            .struct_w_generic_in_array(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // struct that has the generic in a tuple
        let arg1 = StructWTupleGeneric { a: (1, 2) };

        let result = contract_methods
            .struct_w_generic_in_tuple(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        // enum with generic in variant
        let arg1 = EnumWGeneric::B(10);
        let result = contract_methods
            .enum_w_generic(arg1.clone())
            .call()
            .await?
            .value;

        assert_eq!(result, arg1);
    }
    {
        contract_methods
            .unused_generic_args(StructUnusedGeneric::new(15), EnumUnusedGeneric::One(15))
            .call()
            .await?;

        let (the_struct, the_enum) = contract_methods
            .used_and_unused_generic_args(
                StructUsedAndUnusedGenericParams::new(10u8),
                EnumUsedAndUnusedGenericParams::Two(11u8),
            )
            .call()
            .await?
            .value;

        assert_eq!(the_struct.field, 12u8);
        if let EnumUsedAndUnusedGenericParams::Two(val) = the_enum {
            assert_eq!(val, 13)
        } else {
            panic!("Expected the variant EnumUsedAndUnusedGenericParams::Two");
        }
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
            b: vec![(
                [EnumWGeneric::B(StructWTupleGeneric {
                    a: (w_arr_generic.clone(), w_arr_generic),
                })],
                10u32,
            )],
        };
        contract_methods.complex_test(arg1.clone()).call().await?;
    }

    Ok(())
}

#[tokio::test]
async fn contract_vectors() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/vectors"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let methods = contract_instance.methods();

    {
        // vec of u32s
        let arg = vec![0, 1, 2];
        methods.u32_vec(arg).call().await?;
    }
    {
        // vec of vecs of u32s
        let arg = vec![vec![0, 1, 2], vec![0, 1, 2]];
        methods.vec_in_vec(arg.clone()).call().await?;
    }
    {
        // vec of structs
        // ANCHOR: passing_in_vec
        let arg = vec![SomeStruct { a: 0 }, SomeStruct { a: 1 }];
        methods.struct_in_vec(arg.clone()).call().await?;
        // ANCHOR_END: passing_in_vec
    }
    {
        // vec in struct
        let arg = SomeStruct { a: vec![0, 1, 2] };
        methods.vec_in_struct(arg.clone()).call().await?;
    }
    {
        // array in vec
        let arg = vec![[0u64, 1u64], [0u64, 1u64]];
        methods.array_in_vec(arg.clone()).call().await?;
    }
    {
        // vec in array
        let arg = [vec![0, 1, 2], vec![0, 1, 2]];
        methods.vec_in_array(arg.clone()).call().await?;
    }
    {
        // vec in enum
        let arg = SomeEnum::a(vec![0, 1, 2]);
        methods.vec_in_enum(arg.clone()).call().await?;
    }
    {
        // enum in vec
        let arg = vec![SomeEnum::a(0), SomeEnum::a(1)];
        methods.enum_in_vec(arg.clone()).call().await?;
    }
    {
        // tuple in vec
        let arg = vec![(0, 0), (1, 1)];
        methods.tuple_in_vec(arg.clone()).call().await?;
    }
    {
        // vec in tuple
        let arg = (vec![0, 1, 2], vec![0, 1, 2]);
        methods.vec_in_tuple(arg.clone()).call().await?;
    }
    {
        // vec in a vec in a struct in a vec
        let arg = vec![
            SomeStruct {
                a: vec![vec![0, 1, 2], vec![3, 4, 5]],
            },
            SomeStruct {
                a: vec![vec![6, 7, 8], vec![9, 10, 11]],
            },
        ];
        methods
            .vec_in_a_vec_in_a_struct_in_a_vec(arg.clone())
            .call()
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_b256() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/b256"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );

    assert_eq!(
        Bits256([2; 32]),
        contract_instance
            .methods()
            .b256_as_output()
            .call()
            .await?
            .value
    );

    {
        // ANCHOR: 256_arg
        let b256 = Bits256([1; 32]);

        let call_handler = contract_instance.methods().b256_as_input(b256);
        // ANCHOR_END: 256_arg

        assert!(call_handler.call().await?.value);
    }

    Ok(())
}

#[tokio::test]
async fn test_b512() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/b512"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    // ANCHOR: b512_example
    let hi_bits = Bits256::from_hex_str(
        "0xbd0c9b8792876713afa8bff383eebf31c43437823ed761cc3600d0016de5110c",
    )?;
    let lo_bits = Bits256::from_hex_str(
        "0x44ac566bd156b4fc71a4a4cb2655d3dd360c695edb17dc3b64d611e122fea23d",
    )?;
    let b512 = B512::from((hi_bits, lo_bits));
    // ANCHOR_END: b512_example

    assert_eq!(b512, contract_methods.b512_as_output().call().await?.value);

    {
        let lo_bits2 = Bits256::from_hex_str(
            "0x54ac566bd156b4fc71a4a4cb2655d3dd360c695edb17dc3b64d611e122fea23d",
        )?;
        let b512 = B512::from((hi_bits, lo_bits2));

        assert!(contract_methods.b512_as_input(b512).call().await?.value);
    }

    Ok(())
}

fn u128_from(parts: (u64, u64)) -> u128 {
    let bytes: [u8; 16] = [parts.0.to_be_bytes(), parts.1.to_be_bytes()]
        .concat()
        .try_into()
        .unwrap();
    u128::from_be_bytes(bytes)
}

#[tokio::test]
async fn test_u128() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/u128"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();
    {
        let arg = u128_from((1, 2));

        let actual = contract_methods.u128_sum_and_ret(arg).call().await?.value;

        let expected = arg + u128_from((3, 4));

        assert_eq!(expected, actual);
    }
    {
        let actual = contract_methods.u128_in_enum_output().call().await?.value;

        let expected = SomeEnum::B(u128_from((4, 4)));
        assert_eq!(expected, actual);
    }
    {
        let input = SomeEnum::B(u128_from((3, 3)));

        contract_methods.u128_in_enum_input(input).call().await?;
    }

    Ok(())
}

fn u256_from(parts: (u64, u64, u64, u64)) -> U256 {
    let bytes: [u8; 32] = [
        parts.0.to_be_bytes(),
        parts.1.to_be_bytes(),
        parts.2.to_be_bytes(),
        parts.3.to_be_bytes(),
    ]
    .concat()
    .try_into()
    .unwrap();
    U256::from(bytes)
}

#[tokio::test]
async fn test_u256() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TypesContract",
            project = "e2e/sway/types/contracts/u256"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TypesContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();
    {
        let arg = u256_from((1, 2, 3, 4));
        let actual = contract_methods.u256_sum_and_ret(arg).call().await?.value;
        let expected = arg + u256_from((3, 4, 5, 6));

        assert_eq!(expected, actual);
    }
    {
        let actual = contract_methods.u256_in_enum_output().call().await?.value;
        let expected = SomeEnum::B(u256_from((1, 2, 3, 4)));

        assert_eq!(expected, actual);
    }
    {
        let input = SomeEnum::B(u256_from((2, 3, 4, 5)));

        contract_methods.u256_in_enum_input(input).call().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_base_type_in_vec_output() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "VectorOutputContract",
            project = "e2e/sway/types/contracts/vector_output"
        )),
        Deploy(
            name = "contract_instance",
            contract = "VectorOutputContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    // ANCHOR: returning_vec
    let response = contract_methods.u8_in_vec(10).call().await?;
    assert_eq!(response.value, (0..10).collect::<Vec<_>>());
    // ANCHOR_END: returning_vec

    let response = contract_methods.u16_in_vec(11).call().await?;
    assert_eq!(response.value, (0..11).collect::<Vec<_>>());

    let response = contract_methods.u32_in_vec(12).call().await?;
    assert_eq!(response.value, (0..12).collect::<Vec<_>>());

    let response = contract_methods.u64_in_vec(13).call().await?;
    assert_eq!(response.value, (0..13).collect::<Vec<_>>());

    let response = contract_methods.bool_in_vec().call().await?;
    assert_eq!(response.value, [true, false, true, false].to_vec());

    let response = contract_methods.b256_in_vec(13).call().await?;
    assert_eq!(response.value, vec![Bits256([2; 32]); 13]);

    Ok(())
}

#[tokio::test]
async fn test_composite_types_in_vec_output() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "VectorOutputContract",
            project = "e2e/sway/types/contracts/vector_output"
        )),
        Deploy(
            name = "contract_instance",
            contract = "VectorOutputContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let expected: Vec<[u64; 4]> = vec![[1, 1, 1, 1], [2, 2, 2, 2], [3, 3, 3, 3], [4, 4, 4, 4]];
        let response = contract_methods.array_in_vec().call().await?.value;
        assert_eq!(response, expected);
    }
    {
        let expected: Vec<Pasta> = vec![
            Pasta::Tortelini(Bimbam {
                bim: 1111,
                bam: 2222_u32,
            }),
            Pasta::Rigatoni(1987),
            Pasta::Spaghetti(true),
        ];

        let response = contract_methods.enum_in_vec().call().await?.value;
        assert_eq!(response, expected);
    }

    {
        let expected: Vec<Bimbam> = vec![
            Bimbam {
                bim: 1111,
                bam: 2222_u32,
            },
            Bimbam {
                bim: 3333,
                bam: 4444_u32,
            },
            Bimbam {
                bim: 5555,
                bam: 6666_u32,
            },
        ];
        let response = contract_methods.struct_in_vec().call().await?.value;
        assert_eq!(response, expected);
    }

    {
        let expected: Vec<(u64, u32)> = vec![(1111, 2222_u32), (3333, 4444_u32), (5555, 6666_u32)];
        let response = contract_methods.tuple_in_vec().call().await?.value;
        assert_eq!(response, expected);
    }

    {
        let expected: Vec<SizedAsciiString<4>> =
            vec!["hell".try_into()?, "ello".try_into()?, "lloh".try_into()?];
        let response = contract_methods.str_in_vec().call().await?.value;
        assert_eq!(response, expected);
    }
    Ok(())
}

#[tokio::test]
async fn test_bytes_output() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "BytesOutputContract",
            project = "e2e/sway/types/contracts/bytes"
        )),
        Deploy(
            name = "contract_instance",
            contract = "BytesOutputContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.return_bytes(10).call().await?;

    assert_eq!(response.value, (0..10).collect::<Vec<_>>());

    Ok(())
}

#[tokio::test]
async fn test_bytes_as_input() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "BytesInputContract",
            project = "e2e/sway/types/contracts/bytes"
        )),
        Deploy(
            name = "contract_instance",
            contract = "BytesInputContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        // ANCHOR: bytes_arg
        let bytes = Bytes(vec![40, 41, 42]);

        contract_methods.accept_bytes(bytes).call().await?;
        // ANCHOR_END: bytes_arg
    }
    {
        let bytes = Bytes(vec![40, 41, 42]);
        let wrapper = Wrapper {
            inner: vec![bytes.clone(), bytes.clone()],
            inner_enum: SomeEnum::Second(bytes),
        };

        contract_methods.accept_nested_bytes(wrapper).call().await?;
    }

    Ok(())
}

#[tokio::test]
async fn contract_raw_slice() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RawSliceContract",
            project = "e2e/sway/types/contracts/raw_slice"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RawSliceContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    {
        for length in 0u8..=10 {
            let response = contract_methods.return_raw_slice(length).call().await?;
            assert_eq!(response.value, (0u8..length).collect::<Vec<u8>>());
        }
    }
    {
        contract_methods
            .accept_raw_slice(RawSlice(vec![40, 41, 42]))
            .call()
            .await?;
    }
    {
        let raw_slice = RawSlice(vec![40, 41, 42]);
        let wrapper = Wrapper {
            inner: vec![raw_slice.clone(), raw_slice.clone()],
            inner_enum: SomeEnum::Second(raw_slice),
        };

        contract_methods
            .accept_nested_raw_slice(wrapper)
            .call()
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn contract_string_slice() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "StringSliceContract",
            project = "e2e/sway/types/contracts/string_slice"
        )),
        Deploy(
            name = "contract_instance",
            contract = "StringSliceContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    let response = contract_methods
        .handles_str("contract-input".try_into()?)
        .call()
        .await?;
    assert_eq!(response.value, "contract-return");

    Ok(())
}

#[tokio::test]
async fn contract_std_lib_string() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "StdLibString",
            project = "e2e/sway/types/contracts/std_lib_string"
        )),
        Deploy(
            name = "contract_instance",
            contract = "StdLibString",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let resp = contract_methods.return_dynamic_string().call().await?.value;
        assert_eq!(resp, "Hello World");
    }
    {
        let _resp = contract_methods
            .accepts_dynamic_string(String::from("Hello World"))
            .call()
            .await?;
    }
    {
        // confirm encoding/decoding a string wasn't faulty and led to too high gas consumption
        let _resp = contract_methods
            .echoes_dynamic_string(String::from("Hello Fuel"))
            .with_tx_policies(TxPolicies::default().with_script_gas_limit(3600))
            .call()
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_heap_type_in_enums() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "HeapTypeInEnum",
            project = "e2e/sway/types/contracts/heap_type_in_enums"
        )),
        Deploy(
            name = "contract_instance",
            contract = "HeapTypeInEnum",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let resp = contract_methods.returns_bytes_result(true).call().await?;
        let expected = Ok(Bytes(vec![1, 1, 1, 1]));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_bytes_result(false).call().await?;
        let expected = Err(TestError::Something([255u8, 255u8, 255u8, 255u8, 255u8]));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_vec_result(true).call().await?;
        let expected = Ok(vec![2, 2, 2, 2, 2]);

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_vec_result(false).call().await?;
        let expected = Err(TestError::Else(7777));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_string_result(true).call().await?;
        let expected = Ok("Hello World".to_string());

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_string_result(false).call().await?;
        let expected = Err(TestError::Else(3333));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_str_result(true).call().await?;
        let expected = Ok("Hello World".try_into()?);

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_string_result(false).call().await?;
        let expected = Err(TestError::Else(3333));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_bytes_option(true).call().await?;
        let expected = Some(Bytes(vec![1, 1, 1, 1]));

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_bytes_option(false).call().await?;

        assert!(resp.value.is_none());
    }
    {
        let resp = contract_methods.returns_vec_option(true).call().await?;
        let expected = Some(vec![2, 2, 2, 2, 2]);

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_vec_option(false).call().await?;

        assert!(resp.value.is_none());
    }
    {
        let resp = contract_methods.returns_string_option(true).call().await?;
        let expected = Some("Hello World".to_string());

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_string_option(false).call().await?;

        assert!(resp.value.is_none());
    }
    {
        let resp = contract_methods.returns_str_option(true).call().await?;
        let expected = Some("Hello World".try_into()?);

        assert_eq!(resp.value, expected);
    }
    {
        let resp = contract_methods.returns_string_option(false).call().await?;

        assert!(resp.value.is_none());
    }

    Ok(())
}

#[tokio::test]
async fn nested_heap_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "HeapTypeInEnum",
            project = "e2e/sway/types/contracts/heap_types"
        )),
        Deploy(
            name = "contract_instance",
            contract = "HeapTypeInEnum",
            wallet = "wallet"
        ),
    );

    let arr = [2u8, 4, 8];
    let struct_generics = StructGenerics {
        one: Bytes(arr.to_vec()),
        two: String::from("fuel"),
        three: RawSlice(arr.to_vec()),
    };

    let enum_vec = [struct_generics.clone(), struct_generics].to_vec();
    let expected = EnumGeneric::One(enum_vec);

    let result = contract_instance
        .methods()
        .nested_heap_types()
        .call()
        .await?;

    assert_eq!(result.value, expected);

    Ok(())
}
