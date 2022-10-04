use fuels::prelude::*;

#[tokio::test]
async fn can_handle_function_called_new() -> anyhow::Result<()> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contract_instance/collision_in_fn_names"
    );

    let response = contract_instance.methods().new().call().await?.value;

    assert_eq!(response, 12345);
    Ok(())
}

#[tokio::test]
async fn test_contract_setup_macro_deploy_with_salt() -> Result<(), Error> {
    // ANCHOR: contract_setup_macro_multi
    // The first wallet name must be `wallet`
    setup_contract_test!(
        foo_contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/foo_contract"
    );
    let foo_contract_id = foo_contract_instance.get_contract_id();

    // The macros that want to use the `wallet` have to set
    // the wallet name to `None`
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/behaviors/foo_caller_contract"
    );
    let foo_caller_contract_id = foo_caller_contract_instance.get_contract_id();

    setup_contract_test!(
        foo_caller_contract_instance2,
        None,
        "packages/fuels/tests/behaviors/foo_caller_contract"
    );
    let foo_caller_contract_id2 = foo_caller_contract_instance2.get_contract_id();

    // Because we deploy with salt, we can deploy the same contract multiple times
    assert_ne!(foo_caller_contract_id, foo_caller_contract_id2);

    // The first contract can be called because they were deployed on the same provider
    let bits = *foo_contract_id.hash();
    let res = foo_caller_contract_instance
        .methods()
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    assert!(!res.value);

    let res = foo_caller_contract_instance2
        .methods()
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    assert!(!res.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contract_instance/collision_in_fn_names"
    );

    assert_eq!(contract_instance.get_wallet().address(), wallet.address());
    //`get_contract_id()` is tested in
    // async fn test_contract_calling_contract() -> Result<(), Error> {
    Ok(())
}

#[tokio::test]
async fn test_connect_wallet() -> anyhow::Result<()> {
    // ANCHOR: contract_setup_macro_manual_wallet
    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_contract_test!(
        contract_instance,
        None,
        "packages/fuels/tests/behaviors/contract_test"
    );
    // ANCHOR_END: contract_setup_macro_manual_wallet

    // pay for call with wallet
    let tx_params = TxParameters::new(Some(10), Some(10000), None);
    contract_instance
        .methods()
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm that funds have been deducted
    let wallet_balance = wallet.get_asset_balance(&Default::default()).await?;
    assert!(DEFAULT_COIN_AMOUNT > wallet_balance);

    // pay for call with wallet_2
    contract_instance
        .with_wallet(wallet_2.clone())?
        .methods()
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm there are no changes to wallet, wallet_2 has been charged
    let wallet_balance_second_call = wallet.get_asset_balance(&Default::default()).await?;
    let wallet_2_balance = wallet_2.get_asset_balance(&Default::default()).await?;
    assert_eq!(wallet_balance_second_call, wallet_balance);
    assert!(DEFAULT_COIN_AMOUNT > wallet_2_balance);
    Ok(())
}
