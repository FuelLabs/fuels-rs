use fuels::{
    core::codec::EncoderConfig,
    prelude::*,
    types::{Bits256, SizedAsciiString, U256},
};

#[tokio::test]
async fn contract_default_configurables() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/configurables/out/release/configurables-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_id = Contract::load_from(
        "sway/contracts/configurables/out/release/configurables.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .return_configurables()
        .call()
        .await?;

    let expected_value = (
        true,
        8,
        16,
        32,
        63,
        U256::from(8),
        Bits256([1; 32]),
        "fuel".try_into()?,
        (8, true),
        [253, 254, 255],
        StructWithGeneric {
            field_1: 8u8,
            field_2: 16,
        },
        EnumWithGeneric::VariantOne(true),
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn script_default_configurables() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_configurables"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let mut script_instance = script_instance;
    script_instance.convert_into_loader().await?;

    let response = script_instance.main().call().await?;

    let expected_value = (
        true,
        8,
        16,
        32,
        63,
        U256::from(8),
        Bits256([1; 32]),
        "fuel".try_into()?,
        (8, true),
        [253, 254, 255],
        StructWithGeneric {
            field_1: 8u8,
            field_2: 16,
        },
        EnumWithGeneric::VariantOne(true),
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn contract_configurables() -> Result<()> {
    // ANCHOR: contract_configurables
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/configurables/out/release/configurables-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let str_4: SizedAsciiString<4> = "FUEL".try_into()?;
    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyContractConfigurables::default()
        .with_BOOL(false)?
        .with_U8(7)?
        .with_U16(15)?
        .with_U32(31)?
        .with_U64(63)?
        .with_U256(U256::from(8))?
        .with_B256(Bits256([2; 32]))?
        .with_STR_4(str_4.clone())?
        .with_TUPLE((7, false))?
        .with_ARRAY([252, 253, 254])?
        .with_STRUCT(new_struct.clone())?
        .with_ENUM(new_enum.clone())?;

    let contract_id = Contract::load_from(
        "sway/contracts/configurables/out/release/configurables.bin",
        LoadConfiguration::default().with_configurables(configurables),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());
    // ANCHOR_END: contract_configurables

    let response = contract_instance
        .methods()
        .return_configurables()
        .call()
        .await?;

    let expected_value = (
        false,
        7,
        15,
        31,
        63,
        U256::from(8),
        Bits256([2; 32]),
        str_4,
        (7, false),
        [252, 253, 254],
        new_struct,
        new_enum,
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn contract_manual_configurables() -> Result<()> {
    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/configurables"
        )),
        Wallets("wallet")
    );

    let str_4: SizedAsciiString<4> = "FUEL".try_into()?;
    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyContractConfigurables::default()
        .with_BOOL(false)?
        .with_U8(7)?
        .with_U16(15)?
        .with_U32(31)?
        .with_U64(63)?
        .with_U256(U256::from(8))?
        .with_B256(Bits256([2; 32]))?
        .with_STR_4(str_4.clone())?
        .with_TUPLE((7, false))?
        .with_ARRAY([252, 253, 254])?
        .with_STRUCT(new_struct.clone())?
        .with_ENUM(new_enum.clone())?;

    let contract_id = Contract::load_from(
        "sway/contracts/configurables/out/release/configurables.bin",
        LoadConfiguration::default(),
    )?
    .with_configurables(configurables)
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .return_configurables()
        .call()
        .await?;

    let expected_value = (
        false,
        7,
        15,
        31,
        63,
        U256::from(8),
        Bits256([2; 32]),
        str_4,
        (7, false),
        [252, 253, 254],
        new_struct,
        new_enum,
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn script_configurables() -> Result<()> {
    // ANCHOR: script_configurables
    abigen!(Script(
        name = "MyScript",
        abi = "e2e/sway/scripts/script_configurables/out/release/script_configurables-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;
    let bin_path = "sway/scripts/script_configurables/out/release/script_configurables.bin";
    let instance = MyScript::new(wallet, bin_path);

    let str_4: SizedAsciiString<4> = "FUEL".try_into()?;
    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyScriptConfigurables::new(EncoderConfig {
        max_tokens: 5,
        ..Default::default()
    })
    .with_BOOL(false)?
    .with_U8(7)?
    .with_U16(15)?
    .with_U32(31)?
    .with_U64(63)?
    .with_U256(U256::from(8))?
    .with_B256(Bits256([2; 32]))?
    .with_STR_4(str_4.clone())?
    .with_TUPLE((7, false))?
    .with_ARRAY([252, 253, 254])?
    .with_STRUCT(new_struct.clone())?
    .with_ENUM(new_enum.clone())?;

    let response = instance
        .with_configurables(configurables)
        .main()
        .call()
        .await?;
    // ANCHOR_END: script_configurables

    let expected_value = (
        false,
        7,
        15,
        31,
        63,
        U256::from(8),
        Bits256([2; 32]),
        str_4,
        (7, false),
        [252, 253, 254],
        new_struct,
        new_enum,
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn configurable_encoder_config_is_applied() {
    abigen!(Script(
        name = "MyScript",
        abi = "e2e/sway/scripts/script_configurables/out/release/script_configurables-abi.json"
    ));

    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };

    {
        let _configurables = MyScriptConfigurables::default()
            .with_STRUCT(new_struct.clone())
            .expect("no encoder config, it works");
    }
    {
        let encoder_config = EncoderConfig {
            max_tokens: 1,
            ..Default::default()
        };

        // Fails when a wrong encoder config is set
        let configurables_error = MyScriptConfigurables::new(encoder_config)
            .with_STRUCT(new_struct)
            .expect_err("should error");

        assert!(configurables_error
            .to_string()
            .contains("token limit `1` reached while encoding. Try increasing it"),)
    }
}
