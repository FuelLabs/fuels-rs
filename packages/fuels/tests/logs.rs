use fuels::prelude::*;

#[tokio::test]
async fn test_parse_logged_varibles() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    // ANCHOR: produce_logs
    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_variables().call().await?;

    let log_u64 = response.get_logs_with_type::<u64>()?;
    let log_bits256 = response.get_logs_with_type::<Bits256>()?;
    let log_string = response.get_logs_with_type::<SizedAsciiString<4>>()?;
    let log_array = response.get_logs_with_type::<[u8; 3]>()?;

    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);

    assert_eq!(log_u64, vec![64]);
    assert_eq!(log_bits256, vec![expected_bits256]);
    assert_eq!(log_string, vec!["Fuel"]);
    assert_eq!(log_array, vec![[1, 2, 3]]);
    // ANCHOR_END: produce_logs

    Ok(())
}

#[tokio::test]
async fn test_parse_logs_values() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_values().call().await?;

    let log_u64 = response.get_logs_with_type::<u64>()?;
    let log_u32 = response.get_logs_with_type::<u32>()?;
    let log_u16 = response.get_logs_with_type::<u16>()?;
    let log_u8 = response.get_logs_with_type::<u8>()?;
    // try to retrieve non existent log
    let log_nonexistent = response.get_logs_with_type::<bool>()?;

    assert_eq!(log_u64, vec![64]);
    assert_eq!(log_u32, vec![32]);
    assert_eq!(log_u16, vec![16]);
    assert_eq!(log_u8, vec![8]);
    assert!(log_nonexistent.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_parse_logs_custom_types() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_custom_types().call().await?;

    let log_test_struct = response.get_logs_with_type::<TestStruct>()?;
    let log_test_enum = response.get_logs_with_type::<TestEnum>()?;

    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo();

    assert_eq!(log_test_struct, vec![expected_struct]);
    assert_eq!(log_test_enum, vec![expected_enum]);

    Ok(())
}

#[tokio::test]
async fn test_parse_logs_generic_types() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_generic_types().call().await?;

    let log_struct = response.get_logs_with_type::<StructWithGeneric<[_; 3]>>()?;
    let log_enum = response.get_logs_with_type::<EnumWithGeneric<[_; 3]>>()?;
    let log_struct_nested =
        response.get_logs_with_type::<StructWithNestedGeneric<StructWithGeneric<[_; 3]>>>()?;
    let log_struct_deeply_nested = response.get_logs_with_type::<StructDeeplyNestedGeneric<
        StructWithNestedGeneric<StructWithGeneric<[_; 3]>>,
    >>()?;

    let l = [1u8, 2u8, 3u8];
    let expected_struct = StructWithGeneric {
        field_1: l,
        field_2: 64,
    };
    let expected_enum = EnumWithGeneric::VariantOne(l);
    let expected_nested_struct = StructWithNestedGeneric {
        field_1: expected_struct.clone(),
        field_2: 64,
    };
    let expected_deeply_nested_struct = StructDeeplyNestedGeneric {
        field_1: expected_nested_struct.clone(),
        field_2: 64,
    };

    assert_eq!(log_struct, vec![expected_struct]);
    assert_eq!(log_enum, vec![expected_enum]);
    assert_eq!(log_struct_nested, vec![expected_nested_struct]);
    assert_eq!(
        log_struct_deeply_nested,
        vec![expected_deeply_nested_struct]
    );

    Ok(())
}

#[tokio::test]
async fn test_get_logs() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    // ANCHOR: get_logs
    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_multiple_logs().call().await?;
    let logs = response.get_logs()?;
    // ANCHOR_END: get_logs

    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo();
    let expected_generic_struct = StructWithGeneric {
        field_1: expected_struct.clone(),
        field_2: 64,
    };
    let expected_logs: Vec<String> = vec![
        format!("{:?}", 64u64),
        format!("{:?}", 32u32),
        format!("{:?}", 16u16),
        format!("{:?}", 8u8),
        format!("{:?}", 64u64),
        format!("{:?}", expected_bits256),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
        format!("{:?}", expected_struct),
        format!("{:?}", expected_enum),
        format!("{:?}", expected_generic_struct),
    ];

    assert_eq!(logs, expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_get_logs_with_no_logs() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
    );

    let contract_methods = contract_instance.methods();
    let logs = contract_methods
        .initialize_counter(42)
        .call()
        .await?
        .get_logs()?;

    assert!(logs.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_multi_call_log_single_contract() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    let contract_methods = contract_instance.methods();

    let call_handler_1 = contract_methods.produce_logs_values();
    let call_handler_2 = contract_methods.produce_logs_variables();

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let expected_logs: Vec<String> = vec![
        format!("{:?}", 64u64),
        format!("{:?}", 32u32),
        format!("{:?}", 16u16),
        format!("{:?}", 8u8),
        format!("{:?}", 64u64),
        format!(
            "{:?}",
            Bits256([
                239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161,
                16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
            ])
        ),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
    ];

    let logs = multi_call_handler.call::<((), ())>().await?.get_logs()?;

    assert_eq!(logs, expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_log_multiple_contracts() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/logs/contract_logs"
    );

    setup_contract_test!(
        contract_instance2,
        None,
        "packages/fuels/tests/logs/contract_logs"
    );

    let call_handler_1 = contract_instance.methods().produce_logs_values();
    let call_handler_2 = contract_instance2.methods().produce_logs_variables();

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let expected_logs: Vec<String> = vec![
        format!("{:?}", 64u64),
        format!("{:?}", 32u32),
        format!("{:?}", 16u16),
        format!("{:?}", 8u8),
        format!("{:?}", 64u64),
        format!(
            "{:?}",
            Bits256([
                239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161,
                16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
            ])
        ),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
    ];

    let logs = multi_call_handler.call::<((), ())>().await?.get_logs()?;

    assert_eq!(logs, expected_logs);

    Ok(())
}

fn assert_is_revert_containing_msg(msg: &str, error: Error) {
    assert!(matches!(error, Error::RevertTransactionError(..)));
    if let Error::RevertTransactionError(error_message, _) = error {
        assert!(error_message.contains(msg));
    }
}

#[tokio::test]
async fn test_require_log() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/require"
    );

    let contract_methods = contract_instance.methods();
    {
        let error = contract_methods
            .require_primitive()
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("42", error);
    }
    {
        let error = contract_methods
            .require_string()
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("fuel", error);
    }
    {
        let error = contract_methods
            .require_custom_generic()
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}

#[tokio::test]
async fn test_multi_call_require_log_single_contract() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/require"
    );

    let contract_methods = contract_instance.methods();

    // The output of the error depends on the order of the contract
    // handlers as the script returns the first revert it finds.
    {
        let call_handler_1 = contract_methods.require_string();
        let call_handler_2 = contract_methods.require_custom_generic();

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("fuel", error);
    }
    {
        let call_handler_1 = contract_methods.require_custom_generic();
        let call_handler_2 = contract_methods.require_string();

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}

#[tokio::test]
async fn test_multi_call_require_log_multi_contract() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/require"
    );

    setup_contract_test!(
        contract_instance2,
        None,
        "packages/fuels/tests/contracts/require"
    );

    let contract_methods = contract_instance.methods();
    let contract_methods2 = contract_instance2.methods();

    // The output of the error depends on the order of the contract
    // handlers as the script returns the first revert it finds.
    {
        let call_handler_1 = contract_methods.require_string();
        let call_handler_2 = contract_methods2.require_custom_generic();

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("fuel", error);
    }
    {
        let call_handler_1 = contract_methods2.require_custom_generic();
        let call_handler_2 = contract_methods.require_string();

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn test_script_get_logs() -> Result<(), Error> {
    // ANCHOR: script_logs
    script_abigen!(
        log_script,
        "packages/fuels/tests/logs/script_logs/out/debug/script_logs-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/logs/script_logs/out/debug/script_logs.bin";
    let instance = log_script::new(wallet.clone(), bin_path);

    let response = instance.main().call().await?;

    let logs = response.get_logs()?;
    let log_u64 = response.get_logs_with_type::<u64>()?;
    // ANCHOR_END: script_logs

    let l = [1u8, 2u8, 3u8];
    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo();
    let expected_generic_struct = StructWithGeneric {
        field_1: expected_struct.clone(),
        field_2: 64,
    };

    let expected_generic_enum = EnumWithGeneric::VariantOne(l);
    let expected_nested_struct = StructWithNestedGeneric {
        field_1: expected_generic_struct.clone(),
        field_2: 64,
    };
    let expected_deeply_nested_struct = StructDeeplyNestedGeneric {
        field_1: expected_nested_struct.clone(),
        field_2: 64,
    };
    let expected_logs: Vec<String> = vec![
        format!("{:?}", 128u64),
        format!("{:?}", 32u32),
        format!("{:?}", 16u16),
        format!("{:?}", 8u8),
        format!("{:?}", 64u64),
        format!("{:?}", expected_bits256),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
        format!("{:?}", expected_struct),
        format!("{:?}", expected_enum),
        format!("{:?}", expected_generic_struct),
        format!("{:?}", expected_generic_enum),
        format!("{:?}", expected_nested_struct),
        format!("{:?}", expected_deeply_nested_struct),
    ];

    assert_eq!(logs, expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_script_get_logs_with_type() -> Result<(), Error> {
    script_abigen!(
        log_script,
        "packages/fuels/tests/logs/script_logs/out/debug/script_logs-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/logs/script_logs/out/debug/script_logs.bin";
    let instance = log_script::new(wallet.clone(), bin_path);

    let response = instance.main().call().await?;

    let l = [1u8, 2u8, 3u8];
    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo();
    let expected_generic_struct = StructWithGeneric {
        field_1: expected_struct.clone(),
        field_2: 64,
    };

    let expected_generic_enum = EnumWithGeneric::VariantOne(l);
    let expected_nested_struct = StructWithNestedGeneric {
        field_1: expected_generic_struct.clone(),
        field_2: 64,
    };
    let expected_deeply_nested_struct = StructDeeplyNestedGeneric {
        field_1: expected_nested_struct.clone(),
        field_2: 64,
    };

    let log_u64 = response.get_logs_with_type::<u64>()?;
    let log_u32 = response.get_logs_with_type::<u32>()?;
    let log_u16 = response.get_logs_with_type::<u16>()?;
    let log_u8 = response.get_logs_with_type::<u8>()?;
    let log_struct = response.get_logs_with_type::<TestStruct>()?;
    let log_enum = response.get_logs_with_type::<TestEnum>()?;
    let log_generic_struct = response.get_logs_with_type::<StructWithGeneric<TestStruct>>()?;
    let log_generic_enum = response.get_logs_with_type::<EnumWithGeneric<[_; 3]>>()?;
    let log_nested_struct =
        response.get_logs_with_type::<StructWithNestedGeneric<StructWithGeneric<TestStruct>>>()?;
    let log_deeply_nested_struct = response.get_logs_with_type::<StructDeeplyNestedGeneric<
        StructWithNestedGeneric<StructWithGeneric<TestStruct>>,
    >>()?;
    // try to retrieve non existent log
    let log_nonexistent = response.get_logs_with_type::<bool>()?;

    assert_eq!(log_u64, vec![128, 64]);
    assert_eq!(log_u32, vec![32]);
    assert_eq!(log_u16, vec![16]);
    assert_eq!(log_u8, vec![8]);
    assert_eq!(log_struct, vec![expected_struct]);
    assert_eq!(log_enum, vec![expected_enum]);
    assert_eq!(log_generic_struct, vec![expected_generic_struct]);
    assert_eq!(log_generic_enum, vec![expected_generic_enum]);
    assert_eq!(log_nested_struct, vec![expected_nested_struct]);
    assert_eq!(
        log_deeply_nested_struct,
        vec![expected_deeply_nested_struct]
    );
    assert!(log_nonexistent.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_script_require_log() -> Result<(), Error> {
    script_abigen!(
        log_script,
        "packages/fuels/tests/scripts/script_require/out/debug/script_require-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_require/out/debug/script_require.bin";
    let instance = log_script::new(wallet.clone(), bin_path);

    {
        let error = instance
            .main(MatchEnum::RequirePrimitive())
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("42", error);
    }
    {
        let error = instance
            .main(MatchEnum::RequireString())
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("fuel", error);
    }
    {
        let instance = log_script::new(wallet.clone(), bin_path);
        let error = instance
            .main(MatchEnum::RequireCustomGeneric())
            .call()
            .await
            .expect_err("Should return a revert error");

        assert_is_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}
