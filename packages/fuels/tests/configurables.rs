use fuels::prelude::*;
use fuels_types::SizedAsciiString;

#[tokio::test]
async fn contract_uses_default_configurables() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/configurables/out/debug/configurables-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::load_from(
        "tests/contracts/configurables/out/debug/configurables.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxParameters::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .return_configurables()
        .call()
        .await?;

    let expected_value = (
        8u8,
        true,
        [253u32, 254u32, 255u32],
        "fuel".try_into()?,
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
async fn script_uses_default_configurables() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_configurables"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let response = script_instance.main().call().await?;

    let expected_value = (
        8u8,
        true,
        [253u32, 254u32, 255u32],
        "fuel".try_into()?,
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
        abi = "packages/fuels/tests/contracts/configurables/out/debug/configurables-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;

    let new_str: SizedAsciiString<4> = "FUEL".try_into()?;
    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyContractConfigurables::new()
        .set_STR_4(new_str.clone())
        .set_STRUCT(new_struct.clone())
        .set_ENUM(new_enum.clone());

    let contract_id = Contract::load_from(
        "tests/contracts/configurables/out/debug/configurables.bin",
        LoadConfiguration::default().set_configurables(configurables),
    )?
    .deploy(&wallet, TxParameters::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());
    // ANCHOR_END: contract_configurables

    let response = contract_instance
        .methods()
        .return_configurables()
        .call()
        .await?;

    let expected_value = (
        8u8,
        true,
        [253u32, 254u32, 255u32],
        new_str,
        new_struct,
        new_enum,
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}

#[tokio::test]
async fn script_configurables() -> Result<()> {
    // ANCHOR: script_configurables
    abigen!(Script(name="MyScript", abi="packages/fuels/tests/scripts/script_configurables/out/debug/script_configurables-abi.json"));

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_configurables/out/debug/script_configurables.bin";
    let instance = MyScript::new(wallet, bin_path);

    let new_str: SizedAsciiString<4> = "FUEL".try_into()?;
    let new_struct = StructWithGeneric {
        field_1: 16u8,
        field_2: 32,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyScriptConfigurables::new()
        .set_STR_4(new_str.clone())
        .set_STRUCT(new_struct.clone())
        .set_ENUM(new_enum.clone());

    let response = instance
        .with_configurables(configurables)
        .main()
        .call()
        .await?;
    // ANCHOR_END: script_configurables

    let expected_value = (
        8u8,
        true,
        [253u32, 254u32, 255u32],
        new_str,
        new_struct,
        new_enum,
    );

    assert_eq!(response.value, expected_value);

    Ok(())
}
