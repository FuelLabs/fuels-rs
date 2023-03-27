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
    let storage_configuration = StorageConfiguration::default().set_manual_storage(storage_vec);

    let contract_id = Contract::deploy(
        "tests/contracts/storage/out/debug/storage.bin",
        &wallet,
        DeployConfiguration::default().set_storage_configuration(storage_configuration),
    )
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
    let storage_configuration = StorageConfiguration::default().set_storage_path(
        "tests/contracts/storage/out/debug/storage-storage_slots.json".to_string(),
    );

    let contract_id = Contract::deploy(
        "tests/contracts/storage/out/debug/storage.bin",
        &wallet,
        DeployConfiguration::default().set_storage_configuration(storage_configuration),
    )
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
async fn test_init_storage_automatically_bad_json_path() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/storage/out/debug/storage-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;
    let storage_configuration = StorageConfiguration::default().set_storage_path(
        "tests/contracts/storage/out/debug/storage-storage_slts.json".to_string(),
    );

    let response = Contract::deploy(
        "tests/contracts/storage/out/debug/storage.bin",
        &wallet,
        DeployConfiguration::default().set_storage_configuration(storage_configuration),
    )
    .await
    .expect_err("Should fail");

    let expected = "Invalid data:";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}
