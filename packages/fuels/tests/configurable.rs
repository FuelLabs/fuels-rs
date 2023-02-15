use fuels::prelude::*;
use fuels_programs::contract::ReplaceConfigurable;
use fuels_types::SizedAsciiString;

#[tokio::test]
async fn contract_configurables() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/configurable/out/debug/configurable-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;

    let h = MyContractConfigurable {
        STR: "e3lah".try_into()?,
        ARR: [200, 201, 202],
        STR2: "luef".try_into()?,
    };

    let contract_id = Contract::deploy_with_parameters(
        "tests/contracts/configurable/out/debug/configurable.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(Some(
            "tests/contracts/configurable/out/debug/configurable-storage_slots.json".to_string(),
        )),
        h.clone().into(),
        Salt::default(),
    )
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance.methods().something().call().await?;

    dbg!(&response.value);

    assert_eq!(response.value.0, h.STR);

    Ok(())
}

// #[tokio::test]
// async fn main_function_arguments() -> Result<()> {
//     abigen!(Script(name="MyScript", abi="packages/fuels/tests/scripts/script_configurable/out/debug/script_configurable-abi.json"));

//     let wallet = launch_provider_and_get_wallet().await;
//     let bin_path = "../fuels/tests/scripts/script_configurable/out/debug/script_configurable.bin";
//     let instance = MyScript::new(wallet, bin_path);

//     let h = Hal3e { halbu: 10 };

//     assert_eq!(result.value, expected);
//     Ok(())
// }
