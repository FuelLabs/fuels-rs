use fuels::{
    core::{
        codec::{ABIEncoder, ABIFormatter},
        traits::Tokenizable,
    },
    prelude::*,
    programs::{debug::ScriptType, executable::Executable},
};

#[tokio::test]
async fn can_debug_single_call_tx() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/types/contracts/nested_structs"
        ))
    );
    let contract_id = Contract::load_from(
        "sway/types/contracts/nested_structs/out/release/nested_structs.bin",
        Default::default(),
    )?
    .contract_id();

    let call_handler = MyContract::new(contract_id, wallet)
        .methods()
        .check_struct_integrity(AllStruct {
            some_struct: SomeStruct {
                field: 2,
                field_2: true,
            },
        });

    let abi = std::fs::read_to_string(
        "./sway/types/contracts/nested_structs/out/release/nested_structs-abi.json",
    )
    .unwrap();
    let decoder = ABIFormatter::from_json_abi(&abi)?;

    // without gas forwarding
    {
        let tb = call_handler
            .clone()
            .call_params(CallParameters::default().with_amount(10))
            .unwrap()
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            ScriptType::detect(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 1);
        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "check_struct_integrity"
        );
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                call_description.encoded_args.as_slice()
            )?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );
    }

    // with gas forwarding
    {
        let tb = call_handler
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(10)
                    .with_gas_forwarded(20),
            )
            .unwrap()
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            ScriptType::detect(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 1);
        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "check_struct_integrity"
        );
        assert_eq!(call_description.gas_forwarded, Some(20));

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                call_description.encoded_args.as_slice()
            )?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );
    }

    Ok(())
}

#[tokio::test]
async fn can_debug_multi_call_tx() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/types/contracts/nested_structs"
        ))
    );
    let contract_id = Contract::load_from(
        "sway/types/contracts/nested_structs/out/release/nested_structs.bin",
        Default::default(),
    )?
    .contract_id();

    let call1 = MyContract::new(contract_id, wallet.clone())
        .methods()
        .check_struct_integrity(AllStruct {
            some_struct: SomeStruct {
                field: 2,
                field_2: true,
            },
        });

    let call2 = MyContract::new(contract_id, wallet.clone())
        .methods()
        .i_am_called_differently(
            AllStruct {
                some_struct: SomeStruct {
                    field: 2,
                    field_2: true,
                },
            },
            MemoryAddress {
                contract_id,
                function_selector: 123,
                function_data: 456,
            },
        );

    let abi = std::fs::read_to_string(
        "./sway/types/contracts/nested_structs/out/release/nested_structs-abi.json",
    )
    .unwrap();
    let decoder = ABIFormatter::from_json_abi(&abi)?;

    // without gas forwarding
    {
        let first_call = call1
            .clone()
            .call_params(CallParameters::default().with_amount(10))
            .unwrap();

        let second_call = call2
            .clone()
            .call_params(CallParameters::default().with_amount(20))
            .unwrap();

        let tb = CallHandler::new_multi_call(wallet.clone())
            .add_call(first_call)
            .add_call(second_call)
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            ScriptType::detect(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 2);

        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "check_struct_integrity"
        );
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                call_description.encoded_args.as_slice()
            )?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );

        let call_description = &call_descriptions[1];
        let fn_selector = call_description.decode_fn_selector().unwrap();

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 20);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(fn_selector, "i_am_called_differently");
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode_fn_args(&fn_selector, call_description.encoded_args.as_slice())?,
            vec![
                "AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }".to_string(),
                format!(
                    "MemoryAddress {{ contract_id: std::contract_id::ContractId {{ bits: Bits256({:?}) }}, function_selector: 123, function_data: 456 }}",
                    contract_id.as_slice()
                )
            ]
        );
    }

    // with gas forwarding
    {
        let first_call = call1
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(10)
                    .with_gas_forwarded(15),
            )
            .unwrap();

        let second_call = call2
            .clone()
            .call_params(
                CallParameters::default()
                    .with_amount(20)
                    .with_gas_forwarded(25),
            )
            .unwrap();

        let tb = CallHandler::new_multi_call(wallet.clone())
            .add_call(first_call)
            .add_call(second_call)
            .transaction_builder()
            .await
            .unwrap();

        let script = tb.script;
        let script_data = tb.script_data;

        let ScriptType::ContractCall(call_descriptions) =
            ScriptType::detect(&script, &script_data)?
        else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 2);

        let call_description = &call_descriptions[0];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "check_struct_integrity"
        );
        assert_eq!(call_description.gas_forwarded, Some(15));

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                call_description.encoded_args.as_slice()
            )?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );

        let call_description = &call_descriptions[1];

        assert_eq!(call_description.contract_id, contract_id);
        assert_eq!(call_description.amount, 20);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "i_am_called_differently"
        );
        assert_eq!(call_description.gas_forwarded, Some(25));

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                call_description.encoded_args.as_slice()
            )?,
            vec![
                "AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }".to_string(),
                format!(
                    "MemoryAddress {{ contract_id: std::contract_id::ContractId {{ bits: Bits256({:?}) }}, function_selector: 123, function_data: 456 }}",
                    contract_id.as_slice()
                )
            ]
        );
    }

    Ok(())
}

#[tokio::test]
async fn can_debug_sway_script() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_struct"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let tb = script_instance
        .main(MyStruct {
            number: 10,
            boolean: false,
        })
        .transaction_builder()
        .await
        .unwrap();

    let abi =
        std::fs::read_to_string("./sway/scripts/script_struct/out/release/script_struct-abi.json")?;

    let decoder = ABIFormatter::from_json_abi(abi)?;

    let ScriptType::Other(desc) = ScriptType::detect(&tb.script, &tb.script_data).unwrap() else {
        panic!("expected a script")
    };

    assert_eq!(
        decoder.decode_fn_args("main", desc.data.as_slice())?,
        vec!["MyStruct { number: 10, boolean: false }"]
    );

    assert_eq!(
        decoder
            .decode_configurables(desc.data_section().unwrap())
            .unwrap(),
        vec![
            ("A_NUMBER".to_owned(), "11".to_owned()),
            (
                "MY_STRUCT".to_owned(),
                "MyStruct { number: 10, boolean: true }".to_owned()
            ),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn debugs_sway_script_with_no_configurables() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let tb = script_instance
        .main(10, 11)
        .transaction_builder()
        .await
        .unwrap();

    let ScriptType::Other(desc) = ScriptType::detect(&tb.script, &tb.script_data).unwrap() else {
        panic!("expected a script")
    };

    assert!(desc.data_section().is_none());

    Ok(())
}
fn generate_modern_sway_binary(len: usize) -> Vec<u8> {
    assert!(
        len > 24,
        "needs at least 24B to fit in the indicator_of_modern_binary, data & configurables offsets"
    );

    let mut custom_script = vec![0; len];
    let indicator_of_modern_binary = fuel_asm::op::jmpf(0x00, 0x04);

    custom_script[4..8].copy_from_slice(&indicator_of_modern_binary.to_bytes());
    custom_script
}

#[tokio::test]
async fn data_section_offset_not_set_if_out_of_bounds() -> Result<()> {
    let mut custom_script = generate_modern_sway_binary(100);
    custom_script[16..24].copy_from_slice(&u64::MAX.to_be_bytes());

    let ScriptType::Other(desc) = ScriptType::detect(&custom_script, &[]).unwrap() else {
        panic!("expected a script")
    };

    assert!(desc.data_section_offset.is_none());

    Ok(())
}

#[tokio::test]
async fn can_detect_a_loader_script_w_data_section() -> Result<()> {
    setup_program_test!(Abigen(Script(
        name = "MyScript",
        project = "e2e/sway/scripts/script_struct"
    )));

    let script_data = ABIEncoder::default()
        .encode(&[MyStruct {
            number: 10,
            boolean: false,
        }
        .into_token()])
        .unwrap();

    let executable =
        Executable::load_from("sway/scripts/script_struct/out/release/script_struct.bin")
            .unwrap()
            .convert_to_loader()
            .unwrap();

    let expected_blob_id = executable.blob().id();
    let script = executable.code();

    let ScriptType::Loader { script, blob_id } = ScriptType::detect(&script, &script_data).unwrap()
    else {
        panic!("expected a loader script")
    };

    assert_eq!(blob_id, expected_blob_id);

    let decoder = ABIFormatter::from_json_abi(std::fs::read_to_string(
        "./sway/scripts/script_struct/out/release/script_struct-abi.json",
    )?)?;

    assert_eq!(
        decoder.decode_fn_args("main", script.data.as_slice())?,
        vec!["MyStruct { number: 10, boolean: false }"]
    );

    assert_eq!(
        decoder
            .decode_configurables(script.data_section().unwrap())
            .unwrap(),
        vec![
            ("A_NUMBER".to_owned(), "11".to_owned()),
            (
                "MY_STRUCT".to_owned(),
                "MyStruct { number: 10, boolean: true }".to_owned()
            ),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn can_detect_a_loader_script_wo_data_section() -> Result<()> {
    setup_program_test!(Abigen(Script(
        name = "MyScript",
        project = "e2e/sway/scripts/empty"
    )));

    let executable = Executable::load_from("sway/scripts/empty/out/release/empty.bin")
        .unwrap()
        .convert_to_loader()
        .unwrap();

    let expected_blob_id = executable.blob().id();
    let script = executable.code();

    let ScriptType::Loader { blob_id, .. } = ScriptType::detect(&script, &[]).unwrap() else {
        panic!("expected a loader script")
    };

    assert_eq!(blob_id, expected_blob_id);

    Ok(())
}
