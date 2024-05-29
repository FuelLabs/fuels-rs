use fuels::{
    core::codec::DecoderConfig,
    prelude::*,
    types::{errors::transaction::Reason, AsciiString, Bits256, SizedAsciiString},
};

#[tokio::test]
async fn test_parse_logged_variables() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    // ANCHOR: produce_logs
    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_variables().call().await?;

    let log_u64 = response.decode_logs_with_type::<u64>()?;
    let log_bits256 = response.decode_logs_with_type::<Bits256>()?;
    let log_string = response.decode_logs_with_type::<SizedAsciiString<4>>()?;
    let log_array = response.decode_logs_with_type::<[u8; 3]>()?;

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
async fn test_parse_logs_values() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_values().call().await?;

    let log_u64 = response.decode_logs_with_type::<u64>()?;
    let log_u32 = response.decode_logs_with_type::<u32>()?;
    let log_u16 = response.decode_logs_with_type::<u16>()?;
    let log_u8 = response.decode_logs_with_type::<u8>()?;
    // try to retrieve non existent log
    let log_nonexistent = response.decode_logs_with_type::<bool>()?;

    assert_eq!(log_u64, vec![64]);
    assert_eq!(log_u32, vec![32]);
    assert_eq!(log_u16, vec![16]);
    assert_eq!(log_u8, vec![8]);
    assert!(log_nonexistent.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_parse_logs_custom_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_custom_types().call().await?;

    let log_test_struct = response.decode_logs_with_type::<TestStruct>()?;
    let log_test_enum = response.decode_logs_with_type::<TestEnum>()?;
    let log_tuple = response.decode_logs_with_type::<(TestStruct, TestEnum)>()?;

    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo;

    assert_eq!(log_test_struct, vec![expected_struct.clone()]);
    assert_eq!(log_test_enum, vec![expected_enum.clone()]);
    assert_eq!(log_tuple, vec![(expected_struct, expected_enum)]);

    Ok(())
}

#[tokio::test]
async fn test_parse_logs_generic_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_logs_generic_types().call().await?;

    let log_struct = response.decode_logs_with_type::<StructWithGeneric<[_; 3]>>()?;
    let log_enum = response.decode_logs_with_type::<EnumWithGeneric<[_; 3]>>()?;
    let log_struct_nested =
        response.decode_logs_with_type::<StructWithNestedGeneric<StructWithGeneric<[_; 3]>>>()?;
    let log_struct_deeply_nested = response.decode_logs_with_type::<StructDeeplyNestedGeneric<
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
async fn test_decode_logs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    // ANCHOR: decode_logs
    let contract_methods = contract_instance.methods();
    let response = contract_methods.produce_multiple_logs().call().await?;
    let logs = response.decode_logs();
    // ANCHOR_END: decode_logs

    let expected_bits256 = Bits256([
        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161, 16, 60,
        239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
    ]);
    let expected_struct = TestStruct {
        field_1: true,
        field_2: expected_bits256,
        field_3: 64,
    };
    let expected_enum = TestEnum::VariantTwo;
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
        format!("{expected_bits256:?}"),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
        format!("{expected_struct:?}"),
        format!("{expected_enum:?}"),
        format!("{expected_generic_struct:?}"),
    ];

    assert_eq!(expected_logs, logs.filter_succeeded());

    Ok(())
}

#[tokio::test]
async fn test_decode_logs_with_no_logs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let logs = contract_methods
        .initialize_counter(42)
        .call()
        .await?
        .decode_logs();

    assert!(logs.filter_succeeded().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_multi_call_log_single_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    let call_handler_1 = contract_methods.produce_logs_values();
    let call_handler_2 = contract_methods.produce_logs_variables();

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
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

    let logs = multi_call_handler.call::<((), ())>().await?.decode_logs();

    assert_eq!(logs.filter_succeeded(), expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_log_multiple_contracts() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/logs/contract_logs"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_instance2",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    let call_handler_1 = contract_instance.methods().produce_logs_values();
    let call_handler_2 = contract_instance2.methods().produce_logs_variables();

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
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

    let logs = multi_call_handler.call::<((), ())>().await?.decode_logs();

    assert_eq!(logs.filter_succeeded(), expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_contract_with_contract_logs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(name = "MyContract", project = "e2e/sway/logs/contract_logs"),
            Contract(
                name = "ContractCaller",
                project = "e2e/sway/logs/contract_with_contract_logs"
            )
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "ContractCaller",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance2",
            contract = "ContractCaller",
            wallet = "wallet"
        ),
    );

    let contract_id = Contract::load_from(
        "./sway/logs/contract_logs/out/release/contract_logs.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id.clone(), wallet.clone());

    let call_handler_1 = contract_caller_instance
        .methods()
        .logs_from_external_contract(contract_id.clone())
        .with_contracts(&[&contract_instance]);

    let call_handler_2 = contract_caller_instance2
        .methods()
        .logs_from_external_contract(contract_id)
        .with_contracts(&[&contract_instance]);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let expected_logs: Vec<String> = vec![
        format!("{:?}", 64),
        format!("{:?}", 32),
        format!("{:?}", 16),
        format!("{:?}", 8),
        format!("{:?}", 64),
        format!("{:?}", 32),
        format!("{:?}", 16),
        format!("{:?}", 8),
    ];

    let logs = multi_call_handler.call::<((), ())>().await?.decode_logs();

    assert_eq!(logs.filter_succeeded(), expected_logs);

    Ok(())
}

fn assert_revert_containing_msg(msg: &str, error: Error) {
    assert!(matches!(error, Error::Transaction(Reason::Reverted { .. })));
    if let Error::Transaction(Reason::Reverted { reason, .. }) = error {
        assert!(
            reason.contains(msg),
            "message: \"{msg}\" not contained in reason: \"{reason}\""
        );
    }
}

#[tokio::test]
async fn test_require_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RequireContract",
            project = "e2e/sway/contracts/require"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RequireContract",
            wallet = "wallet"
        ),
    );

    macro_rules! reverts_with_msg {
        ($method:ident, $execution: ident, $msg:expr) => {
            let error = contract_instance
                .methods()
                .$method()
                .$execution()
                .await
                .expect_err("should return a revert error");

            assert_revert_containing_msg($msg, error);
        };
    }

    reverts_with_msg!(require_primitive, call, "42");
    reverts_with_msg!(require_primitive, simulate, "42");

    reverts_with_msg!(require_string, call, "fuel");
    reverts_with_msg!(require_string, simulate, "fuel");

    reverts_with_msg!(require_custom_generic, call, "StructDeeplyNestedGeneric");
    reverts_with_msg!(
        require_custom_generic,
        simulate,
        "StructDeeplyNestedGeneric"
    );

    reverts_with_msg!(require_with_additional_logs, call, "64");
    reverts_with_msg!(require_with_additional_logs, simulate, "64");

    Ok(())
}

#[tokio::test]
async fn test_multi_call_require_log_single_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RequireContract",
            project = "e2e/sway/contracts/require"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RequireContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    // The output of the error depends on the order of the contract
    // handlers as the script returns the first revert it finds.
    {
        let call_handler_1 = contract_methods.require_string();
        let call_handler_2 = contract_methods.require_custom_generic();

        let mut multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .simulate::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("fuel", error);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("fuel", error);
    }
    {
        let call_handler_1 = contract_methods.require_custom_generic();
        let call_handler_2 = contract_methods.require_string();

        let mut multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .simulate::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("StructDeeplyNestedGeneric", error);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}

#[tokio::test]
async fn test_multi_call_require_log_multi_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RequireContract",
            project = "e2e/sway/contracts/require"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RequireContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_instance2",
            contract = "RequireContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let contract_methods2 = contract_instance2.methods();

    // The output of the error depends on the order of the contract
    // handlers as the script returns the first revert it finds.
    {
        let call_handler_1 = contract_methods.require_string();
        let call_handler_2 = contract_methods2.require_custom_generic();

        let mut multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .simulate::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("fuel", error);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("fuel", error);
    }
    {
        let call_handler_1 = contract_methods2.require_custom_generic();
        let call_handler_2 = contract_methods.require_string();

        let mut multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let error = multi_call_handler
            .simulate::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("StructDeeplyNestedGeneric", error);

        let error = multi_call_handler
            .call::<((), ())>()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("StructDeeplyNestedGeneric", error);
    }

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn test_script_decode_logs() -> Result<()> {
    // ANCHOR: script_logs
    abigen!(Script(
        name = "log_script",
        abi = "e2e/sway/logs/script_logs/out/release/script_logs-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;
    let bin_path = "sway/logs/script_logs/out/release/script_logs.bin";
    let instance = log_script::new(wallet.clone(), bin_path);

    let response = instance.main().call().await?;

    let logs = response.decode_logs();
    let log_u64 = response.decode_logs_with_type::<u64>()?;
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
    let expected_enum = TestEnum::VariantTwo;
    let expected_tuple = (expected_struct.clone(), expected_enum.clone());
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
        format!("{expected_bits256:?}"),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
        format!("{expected_struct:?}"),
        format!("{expected_enum:?}"),
        format!("{expected_tuple:?}"),
        format!("{expected_generic_struct:?}"),
        format!("{expected_generic_enum:?}"),
        format!("{expected_nested_struct:?}"),
        format!("{expected_deeply_nested_struct:?}"),
    ];

    assert_eq!(logs.filter_succeeded(), expected_logs);

    Ok(())
}

#[tokio::test]
async fn test_contract_with_contract_logs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(name = "MyContract", project = "e2e/sway/logs/contract_logs",),
            Contract(
                name = "ContractCaller",
                project = "e2e/sway/logs/contract_with_contract_logs",
            )
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "ContractCaller",
            wallet = "wallet"
        )
    );

    let contract_id = Contract::load_from(
        "./sway/logs/contract_logs/out/release/contract_logs.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id.clone(), wallet.clone());

    let expected_logs: Vec<String> = vec![
        format!("{:?}", 64),
        format!("{:?}", 32),
        format!("{:?}", 16),
        format!("{:?}", 8),
    ];

    let logs = contract_caller_instance
        .methods()
        .logs_from_external_contract(contract_id)
        .with_contracts(&[&contract_instance])
        .call()
        .await?
        .decode_logs();

    assert_eq!(expected_logs, logs.filter_succeeded());

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn test_script_logs_with_contract_logs() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(name = "MyContract", project = "e2e/sway/logs/contract_logs",),
            Script(
                name = "LogScript",
                project = "e2e/sway/logs/script_with_contract_logs"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet"
        ),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );

    let expected_num_contract_logs = 4;

    let expected_script_logs: Vec<String> = vec![
        // Contract logs
        format!("{:?}", 64),
        format!("{:?}", 32),
        format!("{:?}", 16),
        format!("{:?}", 8),
        // Script logs
        format!("{:?}", true),
        format!("{:?}", 42),
        format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
        format!("{:?}", [1, 2, 3]),
    ];

    // ANCHOR: instance_to_contract_id
    let contract_id: ContractId = contract_instance.id().into();
    // ANCHOR_END: instance_to_contract_id

    // ANCHOR: external_contract_ids
    let response = script_instance
        .main(contract_id)
        .with_contract_ids(&[contract_id.into()])
        .call()
        .await?;
    // ANCHOR_END: external_contract_ids

    // ANCHOR: external_contract
    let response = script_instance
        .main(contract_id)
        .with_contracts(&[&contract_instance])
        .call()
        .await?;
    // ANCHOR_END: external_contract

    {
        let num_contract_logs = response
            .receipts
            .iter()
            .filter(|receipt| matches!(receipt, Receipt::LogData { id, .. } | Receipt::Log { id, .. } if *id == contract_id))
            .count();

        assert_eq!(num_contract_logs, expected_num_contract_logs);
    }
    {
        let logs = response.decode_logs();

        assert_eq!(logs.filter_succeeded(), expected_script_logs);
    }

    Ok(())
}

#[tokio::test]
async fn test_script_decode_logs_with_type() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "LogScript",
            project = "e2e/sway/logs/script_logs"
        )),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );

    let response = script_instance.main().call().await?;

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
    let expected_enum = TestEnum::VariantTwo;
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

    let log_u64 = response.decode_logs_with_type::<u64>()?;
    let log_u32 = response.decode_logs_with_type::<u32>()?;
    let log_u16 = response.decode_logs_with_type::<u16>()?;
    let log_u8 = response.decode_logs_with_type::<u8>()?;
    let log_struct = response.decode_logs_with_type::<TestStruct>()?;
    let log_enum = response.decode_logs_with_type::<TestEnum>()?;
    let log_generic_struct = response.decode_logs_with_type::<StructWithGeneric<TestStruct>>()?;
    let log_generic_enum = response.decode_logs_with_type::<EnumWithGeneric<[_; 3]>>()?;
    let log_nested_struct = response
        .decode_logs_with_type::<StructWithNestedGeneric<StructWithGeneric<TestStruct>>>()?;
    let log_deeply_nested_struct = response.decode_logs_with_type::<StructDeeplyNestedGeneric<
        StructWithNestedGeneric<StructWithGeneric<TestStruct>>,
    >>()?;
    // try to retrieve non existent log
    let log_nonexistent = response.decode_logs_with_type::<bool>()?;

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
async fn test_script_require_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "LogScript",
            project = "e2e/sway/scripts/script_require"
        )),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );

    macro_rules! reverts_with_msg {
        ($arg:expr, $execution:ident, $msg:expr) => {
            let error = script_instance
                .main($arg)
                .$execution()
                .await
                .expect_err("should return a revert error");
            assert_revert_containing_msg($msg, error);
        };
    }

    reverts_with_msg!(MatchEnum::RequirePrimitive, call, "42");
    reverts_with_msg!(MatchEnum::RequirePrimitive, simulate, "42");

    reverts_with_msg!(MatchEnum::RequireString, call, "fuel");
    reverts_with_msg!(MatchEnum::RequireString, simulate, "fuel");

    reverts_with_msg!(
        MatchEnum::RequireCustomGeneric,
        call,
        "StructDeeplyNestedGeneric"
    );
    reverts_with_msg!(
        MatchEnum::RequireCustomGeneric,
        simulate,
        "StructDeeplyNestedGeneric"
    );

    reverts_with_msg!(MatchEnum::RequireWithAdditionalLogs, call, "64");
    reverts_with_msg!(MatchEnum::RequireWithAdditionalLogs, simulate, "64");

    Ok(())
}

#[tokio::test]
async fn test_contract_require_from_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "MyContract",
                project = "e2e/sway/contracts/lib_contract",
            ),
            Contract(
                name = "ContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller",
            )
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "ContractCaller",
            wallet = "wallet"
        )
    );

    let contract_id = Contract::load_from(
        "./sway/contracts/lib_contract/out/release/lib_contract.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id.clone(), wallet.clone());

    let error = contract_caller_instance
        .methods()
        .require_from_contract(contract_id)
        .with_contracts(&[&contract_instance])
        .call()
        .await
        .expect_err("should return a revert error");

    assert_revert_containing_msg("require from contract", error);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_contract_require_from_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "MyContract",
                project = "e2e/sway/contracts/lib_contract",
            ),
            Contract(
                name = "ContractLogs",
                project = "e2e/sway/logs/contract_logs",
            ),
            Contract(
                name = "ContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller",
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "ContractLogs",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "ContractCaller",
            wallet = "wallet"
        ),
    );

    let contract_id = Contract::load_from(
        "./sway/contracts/lib_contract/out/release/lib_contract.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let lib_contract_instance = MyContract::new(contract_id.clone(), wallet.clone());

    let call_handler_1 = contract_instance.methods().produce_logs_values();

    let call_handler_2 = contract_caller_instance
        .methods()
        .require_from_contract(contract_id)
        .with_contracts(&[&lib_contract_instance]);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let error = multi_call_handler
        .call::<((), ())>()
        .await
        .expect_err("should return a revert error");

    assert_revert_containing_msg("require from contract", error);

    Ok(())
}

#[tokio::test]
async fn test_script_require_from_contract() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "MyContract",
                project = "e2e/sway/contracts/lib_contract",
            ),
            Script(
                name = "LogScript",
                project = "e2e/sway/scripts/require_from_contract"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet"
        ),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );

    let error = script_instance
        .main(contract_instance.id())
        .with_contracts(&[&contract_instance])
        .call()
        .await
        .expect_err("should return a revert error");

    assert_revert_containing_msg("require from contract", error);

    Ok(())
}

fn assert_assert_eq_containing_msg<T: std::fmt::Debug>(left: T, right: T, error: Error) {
    let msg = format!("left: `\"{left:?}\"`\n right: `\"{right:?}\"`");
    assert_revert_containing_msg(&msg, error)
}

#[tokio::test]
async fn test_contract_asserts_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LogContract",
            project = "e2e/sway/contracts/asserts"
        )),
        Deploy(
            name = "contract_instance",
            contract = "LogContract",
            wallet = "wallet"
        ),
    );

    macro_rules! reverts_with_msg {
        (($($arg: expr,)*), $method:ident, $execution: ident, $msg:expr) => {
            let error = contract_instance
                .methods()
                .$method($($arg,)*)
                .$execution()
                .await
                .expect_err("should return a revert error");
            assert_revert_containing_msg($msg, error);
        };
    }
    {
        reverts_with_msg!((32, 64,), assert_primitive, call, "assertion failed");
        reverts_with_msg!((32, 64,), assert_primitive, simulate, "assertion failed");
    }

    macro_rules! reverts_with_assert_eq_msg {
        (($($arg: expr,)*), $method:ident, $execution: ident, $msg:expr) => {
            let error = contract_instance
                .methods()
                .$method($($arg,)*)
                .call()
                .await
                .expect_err("should return a revert error");
            assert_assert_eq_containing_msg($($arg,)* error);
        }
    }

    {
        reverts_with_assert_eq_msg!((32, 64,), assert_eq_primitive, call, "assertion failed");
        reverts_with_assert_eq_msg!((32, 64,), assert_eq_primitive, simulate, "assertion failed");
    }
    {
        let test_struct = TestStruct {
            field_1: true,
            field_2: 64,
        };

        let test_struct2 = TestStruct {
            field_1: false,
            field_2: 32,
        };

        reverts_with_assert_eq_msg!(
            (test_struct.clone(), test_struct2.clone(),),
            assert_eq_struct,
            call,
            "assertion failed"
        );

        reverts_with_assert_eq_msg!(
            (test_struct.clone(), test_struct2.clone(),),
            assert_eq_struct,
            simulate,
            "assertion failed"
        );
    }
    {
        let test_enum = TestEnum::VariantOne;
        let test_enum2 = TestEnum::VariantTwo;
        reverts_with_assert_eq_msg!(
            (test_enum.clone(), test_enum2.clone(),),
            assert_eq_enum,
            call,
            "assertion failed"
        );

        reverts_with_assert_eq_msg!(
            (test_enum.clone(), test_enum2.clone(),),
            assert_eq_enum,
            simulate,
            "assertion failed"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_script_asserts_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "LogScript",
            project = "e2e/sway/scripts/script_asserts"
        )),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );
    macro_rules! reverts_with_msg {
        ($arg:expr, $execution:ident, $msg:expr) => {
            let error = script_instance
                .main($arg)
                .$execution()
                .await
                .expect_err("should return a revert error");
            assert_revert_containing_msg($msg, error);
        };
    }

    macro_rules! reverts_with_assert_eq_msg {
        ($arg:expr, $execution:ident, $msg:expr) => {
            let error = script_instance
                .main($arg)
                .$execution()
                .await
                .expect_err("should return a revert error");
            assert_revert_containing_msg($msg, error);
        };
    }
    {
        reverts_with_msg!(
            MatchEnum::AssertPrimitive((32, 64)),
            call,
            "assertion failed"
        );
        reverts_with_msg!(
            MatchEnum::AssertPrimitive((32, 64)),
            simulate,
            "assertion failed"
        );
    }
    {
        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqPrimitive((32, 64)),
            call,
            "assertion failed"
        );
        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqPrimitive((32, 64)),
            simulate,
            "assertion failed"
        );
    }
    {
        let test_struct = TestStruct {
            field_1: true,
            field_2: 64,
        };

        let test_struct2 = TestStruct {
            field_1: false,
            field_2: 32,
        };
        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqStruct((test_struct.clone(), test_struct2.clone(),)),
            call,
            "assertion failed"
        );
        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqStruct((test_struct.clone(), test_struct2.clone(),)),
            simulate,
            "assertion failed"
        );
    }
    {
        let test_enum = TestEnum::VariantOne;
        let test_enum2 = TestEnum::VariantTwo;

        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqEnum((test_enum.clone(), test_enum2.clone(),)),
            call,
            "assertion failed"
        );
        reverts_with_assert_eq_msg!(
            MatchEnum::AssertEqEnum((test_enum.clone(), test_enum2.clone(),)),
            simulate,
            "assertion failed"
        );
    }

    Ok(())
}

#[tokio::test]
async fn contract_token_ops_error_messages() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/token_ops"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );
    let contract_methods = contract_instance.methods();

    {
        let contract_id = contract_instance.contract_id();
        let asset_id = contract_id.asset_id(&Bits256::zeroed());
        let address = wallet.address();

        let error = contract_methods
            .transfer(1_000_000, asset_id, address.into())
            .simulate()
            .await
            .expect_err("should return a revert error");
        assert_revert_containing_msg("failed transfer to address", error);

        let error = contract_methods
            .transfer(1_000_000, asset_id, address.into())
            .call()
            .await
            .expect_err("should return a revert error");

        assert_revert_containing_msg("failed transfer to address", error);
    }

    Ok(())
}

#[tokio::test]
async fn test_log_results() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/logs/contract_logs"
        ),),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet"
        )
    );

    let response = contract_instance
        .methods()
        .produce_bad_logs()
        .call()
        .await?;

    let log = response.decode_logs();

    let expected_err = format!(
        "codec: missing log formatter for log_id: `LogId({:?}, \"128\")`, data: `{:?}`. \
         Consider adding external contracts using `with_contracts()`",
        contract_instance.id().hash,
        [0u8; 8]
    );

    let succeeded = log.filter_succeeded();
    let failed = log.filter_failed();
    assert_eq!(succeeded, vec!["123".to_string()]);
    assert_eq!(failed.first().unwrap().to_string(), expected_err);

    Ok(())
}

#[tokio::test]
async fn can_configure_decoder_for_contract_log_decoding() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/needs_custom_decoder"
        ),),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet"
        )
    );

    let methods = contract_instance.methods();
    {
        // Single call: decoding with too low max_tokens fails
        let response = methods
            .i_log_a_1k_el_array()
            .with_decoder_config(DecoderConfig {
                max_tokens: 100,
                ..Default::default()
            })
            .call()
            .await?;

        response.decode_logs_with_type::<[u8; 1000]>().expect_err(
            "Should have failed since there are more tokens than what is supported by default.",
        );

        let logs = response.decode_logs();
        assert!(!logs.filter_failed().is_empty(), "Should have had failed to decode logs since there are more tokens than what is supported by default");
    }
    {
        // Single call: increasing limits makes the test pass
        let response = methods
            .i_log_a_1k_el_array()
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call()
            .await?;

        let logs = response.decode_logs_with_type::<[u8; 1000]>()?;
        assert_eq!(logs, vec![[0u8; 1000]]);

        let logs = response.decode_logs();
        assert!(!logs.filter_succeeded().is_empty());
    }
    {
        // Multi call: decoding with too low max_tokens will fail
        let response = CallHandler::new_multi_call(wallet.clone())
            .add_call(methods.i_log_a_1k_el_array())
            .with_decoder_config(DecoderConfig {
                max_tokens: 100,
                ..Default::default()
            })
            .call::<((),)>()
            .await?;

        response.decode_logs_with_type::<[u8; 1000]>().expect_err(
            "should have failed since there are more tokens than what is supported by default",
        );

        let logs = response.decode_logs();
        assert!(!logs.filter_failed().is_empty(), "should have had failed to decode logs since there are more tokens than what is supported by default");
    }
    {
        // Multi call: increasing limits makes the test pass
        let response = CallHandler::new_multi_call(wallet.clone())
            .add_call(methods.i_log_a_1k_el_array())
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call::<((),)>()
            .await?;

        let logs = response.decode_logs_with_type::<[u8; 1000]>()?;
        assert_eq!(logs, vec![[0u8; 1000]]);

        let logs = response.decode_logs();
        assert!(!logs.filter_succeeded().is_empty());
    }

    Ok(())
}

#[tokio::test]
async fn can_configure_decoder_for_script_log_decoding() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "LogScript",
            project = "e2e/sway/logs/script_needs_custom_decoder_logging"
        )),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );

    {
        // Cannot decode the produced log with too low max_tokens
        let response = script_instance
            .main()
            .with_decoder_config(DecoderConfig {
                max_tokens: 100,
                ..Default::default()
            })
            .call()
            .await?;

        response
            .decode_logs_with_type::<[u8; 1000]>()
            .expect_err("Cannot decode the log with default decoder config");

        let logs = response.decode_logs();
        assert!(!logs.filter_failed().is_empty())
    }
    {
        // When the token limit is bumped log decoding succeeds
        let response = script_instance
            .main()
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call()
            .await?;

        let logs = response.decode_logs_with_type::<[u8; 1000]>()?;
        assert_eq!(logs, vec![[0u8; 1000]]);

        let logs = response.decode_logs();
        assert!(!logs.filter_succeeded().is_empty())
    }

    Ok(())
}

#[tokio::test]
async fn contract_heap_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/logs/contract_logs"
        ),),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet"
        )
    );
    let contract_methods = contract_instance.methods();

    {
        let response = contract_methods.produce_string_slice_log().call().await?;
        let logs = response.decode_logs_with_type::<AsciiString>()?;

        assert_eq!("fuel".to_string(), logs.first().unwrap().to_string());
    }
    {
        let response = contract_methods.produce_string_log().call().await?;
        let logs = response.decode_logs_with_type::<String>()?;

        assert_eq!(vec!["fuel".to_string()], logs);
    }
    {
        let response = contract_methods.produce_bytes_log().call().await?;
        let logs = response.decode_logs_with_type::<Bytes>()?;

        assert_eq!(vec![Bytes("fuel".as_bytes().to_vec())], logs);
    }
    {
        let response = contract_methods.produce_raw_slice_log().call().await?;
        let logs = response.decode_logs_with_type::<RawSlice>()?;

        assert_eq!(vec![RawSlice("fuel".as_bytes().to_vec())], logs);
    }
    {
        let v = [1u16, 2, 3].to_vec();
        let some_enum = EnumWithGeneric::VariantOne(v);
        let other_enum = EnumWithGeneric::VariantTwo;
        let v1 = vec![some_enum.clone(), other_enum, some_enum];
        let expected_vec = vec![vec![v1.clone(), v1]];

        let response = contract_methods.produce_vec_log().call().await?;
        let logs = response.decode_logs_with_type::<Vec<Vec<Vec<EnumWithGeneric<Vec<u16>>>>>>()?;

        assert_eq!(vec![expected_vec], logs);
    }

    Ok(())
}

#[tokio::test]
async fn script_heap_log() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "LogScript",
            project = "e2e/sway/logs/script_heap_logs"
        )),
        LoadScript(
            name = "script_instance",
            script = "LogScript",
            wallet = "wallet"
        )
    );
    let response = script_instance.main().call().await?;

    {
        let logs = response.decode_logs_with_type::<AsciiString>()?;

        assert_eq!("fuel".to_string(), logs.first().unwrap().to_string());
    }
    {
        let logs = response.decode_logs_with_type::<String>()?;

        assert_eq!(vec!["fuel".to_string()], logs);
    }
    {
        let logs = response.decode_logs_with_type::<Bytes>()?;

        assert_eq!(vec![Bytes("fuel".as_bytes().to_vec())], logs);
    }
    {
        let logs = response.decode_logs_with_type::<RawSlice>()?;

        assert_eq!(vec![RawSlice("fuel".as_bytes().to_vec())], logs);
    }
    {
        let v = [1u16, 2, 3].to_vec();
        let some_enum = EnumWithGeneric::VariantOne(v);
        let other_enum = EnumWithGeneric::VariantTwo;
        let v1 = vec![some_enum.clone(), other_enum, some_enum];
        let expected_vec = vec![vec![v1.clone(), v1]];

        let logs = response.decode_logs_with_type::<Vec<Vec<Vec<EnumWithGeneric<Vec<u16>>>>>>()?;

        assert_eq!(vec![expected_vec], logs);
    }

    Ok(())
}
