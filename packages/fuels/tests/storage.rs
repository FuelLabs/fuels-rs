use std::str::FromStr;

use fuels::{
    prelude::*,
    tx::{Bytes32, StorageSlot},
    types::Bits256,
};

#[tokio::test]
async fn test_storage_initialization() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/storage/out/debug/storage-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;

    let key = Bytes32::from([1u8; 32]);
    let value = Bytes32::from([2u8; 32]);
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];
    let storage_configuration = StorageConfiguration::default().add_slot_overrides(storage_vec);

    let contract_id = Contract::load_from(
        "tests/contracts/storage/out/debug/storage.bin",
        LoadConfiguration::default().with_storage_configuration(storage_configuration),
    )?
    .deploy(&wallet, TxParameters::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let result = contract_instance
        .methods()
        .get_value_b256(Bits256(key.into()))
        .call()
        .await?
        .value;
    assert_eq!(result.0, *value);

    Ok(())
}

#[tokio::test]
async fn test_init_storage_automatically() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/storage/out/debug/storage-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::load_from(
        "tests/contracts/storage/out/debug/storage.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxParameters::default())
    .await?;

    let key1 =
        Bytes32::from_str("de9090cb50e71c2588c773487d1da7066d0c719849a7e58dc8b6397a25c567c0")
            .unwrap();
    let key2 =
        Bytes32::from_str("f383b0ce51358be57daa3b725fe44acdb2d880604e367199080b4379c41bb6ed")
            .unwrap();

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

    let value = contract_methods
        .get_value_b256(Bits256(*key1))
        .call()
        .await?
        .value;
    assert_eq!(value.0, [1u8; 32]);

    let value = contract_methods
        .get_value_u64(Bits256(*key2))
        .call()
        .await?
        .value;
    assert_eq!(value, 64);
    Ok(())
}

#[tokio::test]
async fn storage_load_error_messages() {
    {
        let json_path = "tests/contracts/storage/out/debug/no_file_on_path.json";
        let expected_error = format!("Invalid data: file \"{json_path}\" does not exist");

        let error = StorageConfiguration::default()
            .add_slot_overrides_from_file(json_path)
            .expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
    {
        let json_path = "tests/contracts/storage/out/debug/storage.bin";
        let expected_error =
            format!("Invalid data: expected \"{json_path}\" to have '.json' extension");

        let error = StorageConfiguration::default()
            .add_slot_overrides_from_file(json_path)
            .expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
}
