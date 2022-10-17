use fuels::prelude::*;
use fuels_core::tx::{Bytes32, StorageSlot};
use std::str::FromStr;

#[tokio::test]
async fn test_storage_initialization() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: storage_slot_create
    let key = Bytes32::from([1u8; 32]);
    let value = Bytes32::from([2u8; 32]);
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];
    // ANCHOR_END: storage_slot_create

    // ANCHOR: manual_storage
    let contract_id = Contract::deploy_with_parameters(
        "tests/storage/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([0; 32]),
    )
    .await?;
    // ANCHOR_END: manual_storage

    let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

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
async fn test_init_storage_automatically() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: automatic_storage
    let contract_id = Contract::deploy_with_parameters(
        "tests/storage/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(
            Some("tests/storage/contract_storage_test/out/debug/contract_storage_test-storage_slots.json".to_string())),
        Salt::default(),
    )
        .await?;
    // ANCHOR_END: automatic_storage

    let key1 =
        Bytes32::from_str("de9090cb50e71c2588c773487d1da7066d0c719849a7e58dc8b6397a25c567c0")
            .unwrap();
    let key2 =
        Bytes32::from_str("f383b0ce51358be57daa3b725fe44acdb2d880604e367199080b4379c41bb6ed")
            .unwrap();

    let contract_methods = MyContract::new(contract_id.to_string(), wallet.clone()).methods();

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
async fn test_init_storage_automatically_bad_json_path() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/storage/contract_storage_test/out/debug/contract_storage_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let response = Contract::deploy_with_parameters(
        "tests/storage/contract_storage_test/out/debug/contract_storage_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(Some(
            "tests/storage/contract_storage_test/out/debug/contract_storage_test-storage_slts.json"
                .to_string(),
        )),
        Salt::default(),
    )
    .await
    .expect_err("Should fail");

    let expected = "Invalid data:";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}
