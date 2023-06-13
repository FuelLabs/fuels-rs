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
    let storage_configuration = StorageConfiguration::from(storage_vec);

    let contract_id = Contract::load_from(
        "tests/contracts/storage/out/debug/storage.bin",
        LoadConfiguration::default().set_storage_configuration(storage_configuration),
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
    let storage_configuration = StorageConfiguration::load_from(
        "tests/contracts/storage/out/debug/storage-storage_slots.json",
    )?;

    let contract_id = Contract::load_from(
        "tests/contracts/storage/out/debug/storage.bin",
        LoadConfiguration::default().set_storage_configuration(storage_configuration),
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
        let expected_error = format!("Invalid data: file '{json_path}' does not exist");

        let error = StorageConfiguration::load_from(json_path).expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
    {
        let json_path = "tests/contracts/storage/out/debug/storage.bin";
        let expected_error =
            format!("Invalid data: expected `{json_path}` to have '.json' extension");

        let error = StorageConfiguration::load_from(json_path).expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
}

#[tokio::test]
#[cfg(feature = "rocksdb")]
async fn test_created_db() -> Result<()> {
    use fuel_core_client::client::{PageDirection, PaginationRequest};
    use fuels::accounts::fuel_crypto::SecretKey;
    use fuels_accounts::wallet::WalletUnlocked;
    use std::fs;
    use std::path::PathBuf;

    let path =
        PathBuf::from(std::env::var("HOME").expect("HOME env var missing")).join(".spider/db");

    let node_config = Config {
        database_path: path.clone(),
        database_type: DbType::RocksDb,
        ..Config::local_node()
    };

    let mut wallet = WalletUnlocked::new_from_private_key(
        SecretKey::from_str("0x4433d156e8c53bf5b50af07aa95a29436f29a94e0ccc5d58df8e57bdc8583c32")
            .unwrap(),
        None,
    );

    let (provider, _) = setup_test_provider(vec![], vec![], Some(node_config), None).await;

    wallet.set_provider(provider.clone());

    let blocks = provider
        .get_blocks(PaginationRequest {
            cursor: None,
            results: 10,
            direction: PageDirection::Forward,
        })
        .await?
        .results;

    assert_eq!(provider.chain_info().await?.name, "spider");
    assert_eq!(blocks.len(), 3);
    assert_eq!(
        *wallet.get_balances().await?.iter().next().unwrap().1,
        225883
    );
    assert_eq!(
        *wallet.get_balances().await?.iter().next().unwrap().1,
        225883
    );
    assert_eq!(wallet.get_balances().await?.len(), 2);

    fs::remove_dir_all(path.parent().expect("Db parend folder do not exist"))?;

    Ok(())
}
