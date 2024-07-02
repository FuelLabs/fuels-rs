use fuels::{
    prelude::*,
    tx::StorageSlot,
    types::{Bits256, Bytes32},
};

#[tokio::test]
async fn test_storage_initialization() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/storage/out/release/storage-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let key = Bytes32::from([1u8; 32]);
    let value = Bytes32::from([2u8; 32]);
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];
    let storage_configuration = StorageConfiguration::default().add_slot_overrides(storage_vec);

    let contract_id = Contract::load_from(
        "sway/contracts/storage/out/release/storage.bin",
        LoadConfiguration::default().with_storage_configuration(storage_configuration),
    )?
    .deploy(&wallet, TxPolicies::default())
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
        abi = "e2e/sway/contracts/storage/out/release/storage-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_id = Contract::load_from(
        "sway/contracts/storage/out/release/storage.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();
    {
        let key: Bytes32 =
            "d95f4c8d717d52323d34c1118b3f0598a5ec3cabae386887507cabd6dd546a43".parse()?;

        let value = contract_methods
            .get_value_b256(Bits256(*key))
            .call()
            .await?
            .value;

        assert_eq!(value.0, [1u8; 32]);
    }
    {
        let key: Bytes32 =
            "c979570128d5f52725e9a343a7f4992d8ed386d7c8cfd25f1c646c51c2ac6b4b".parse()?;

        let value = contract_methods
            .get_value_u64(Bits256(*key))
            .call()
            .await?
            .value;

        assert_eq!(value, 64);
    }

    Ok(())
}

#[tokio::test]
async fn storage_load_error_messages() {
    {
        let json_path = "sway/contracts/storage/out/release/no_file_on_path.json";
        let expected_error = format!("io: file \"{json_path}\" does not exist");

        let error = StorageConfiguration::default()
            .add_slot_overrides_from_file(json_path)
            .expect_err("should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
    {
        let json_path = "sway/contracts/storage/out/release/storage.bin";
        let expected_error = format!("expected \"{json_path}\" to have '.json' extension");

        let error = StorageConfiguration::default()
            .add_slot_overrides_from_file(json_path)
            .expect_err("should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
}
