use fuel_gql_client::fuel_tx::Receipt;
use fuels::prelude::*;

#[tokio::test]
async fn test_multiple_args() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    // Make sure we can call the contract with multiple arguments
    let contract_methods = contract_instance.methods();
    let response = contract_methods.get(5, 6).call().await?;

    assert_eq!(response.value, 5);

    let t = MyType { x: 5, y: 6 };
    let response = contract_methods.get_alt(t.clone()).call().await?;
    assert_eq!(response.value, t);

    let response = contract_methods.get_single(5).call().await?;
    assert_eq!(response.value, 5);
    Ok(())
}

#[tokio::test]
async fn test_contract_calling_contract() -> Result<(), Error> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    // Load and deploy the first compiled contract
    setup_contract_test!(
        foo_contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/foo_contract"
    );
    let foo_contract_id = foo_contract_instance.get_contract_id();

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.methods().foo(true).call().await?;
    assert!(!res.value);

    // Load and deploy the second compiled contract
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/behaviors/foo_caller_contract"
    );

    // Calls the contract that calls the `FooContract` contract, also just
    // flips the bool value passed to it.
    // ANCHOR: external_contract
    let bits = *foo_contract_id.hash();
    let res = foo_caller_contract_instance
        .methods()
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert!(!res.value);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/revert_transaction_error"
    );

    let response = contract_instance
        .methods()
        .make_transaction_fail(0)
        .call()
        .await;

    assert!(matches!(response, Err(Error::RevertTransactionError(..))));
    Ok(())
}

#[tokio::test]
async fn test_multiple_read_calls() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/multiple_read_calls"
    );

    let contract_methods = contract_instance.methods();
    contract_methods.store(42).call().await?;

    // Use "simulate" because the methods don't actually run a transaction, but just a dry-run
    // We can notice here that, thanks to this, we don't generate a TransactionId collision,
    // even if the transactions are theoretically the same.
    let stored = contract_methods.read(0).simulate().await?;

    assert_eq!(stored.value, 42);

    let stored = contract_methods.read(0).simulate().await?;

    assert_eq!(stored.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_gas_errors() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 1;
    let amount_per_coin = 1_000_000;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);

    setup_contract_test!(
        contract_instance,
        None,
        "packages/fuels/tests/behaviors/contract_test"
    );

    // Test running out of gas. Gas price as `None` will be 0.
    let gas_limit = 100;
    let contract_instace_call = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(gas_limit), None));

    //  Test that the call will use more gas than the gas limit
    let gas_used = contract_instace_call
        .estimate_transaction_cost(None)
        .await?
        .gas_used;
    assert!(gas_used > gas_limit);

    let response = contract_instace_call
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));

    // Test for insufficient base asset amount to pay for the transaction fee
    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(Some(100_000_000_000), None, None))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: Response errors; enough coins could not be found";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_call_param_gas_errors() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1000), None))
        .call_params(CallParameters::new(None, None, Some(1)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Revert transaction error: OutOfGas, receipts:";
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1), None))
        .call_params(CallParameters::new(None, None, Some(1000)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_multi_call() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.initialize_counter(42);
    let call_handler_2 = contract_methods.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let (counter, array): (u64, [u64; 2]) = multi_call_handler.call().await?.value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);
    Ok(())
}

#[tokio::test]
async fn test_gas_forwarded_defaults_to_tx_limit() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let gas_limit = 225883;
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(gas_limit), None))
        .call()
        .await?;

    let gas_forwarded = response
        .receipts
        .iter()
        .find(|r| matches!(r, Receipt::Call { .. }))
        .unwrap()
        .gas()
        .unwrap();

    assert_eq!(gas_limit, gas_forwarded);

    Ok(())
}

#[tokio::test]
async fn test_contract_call_fee_estimation() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let gas_price = 100_000_000;
    let gas_limit = 800;
    let tolerance = 0.2;

    let expected_min_gas_price = 0; // This is the default min_gas_price from the ConsensusParameters
    let expected_gas_used = 710;
    let expected_metered_bytes_size = 720;
    let expected_total_fee = 359;

    let estimated_transaction_cost = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(Some(gas_price), Some(gas_limit), None))
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?;

    assert_eq!(
        estimated_transaction_cost.min_gas_price,
        expected_min_gas_price
    );
    assert_eq!(estimated_transaction_cost.gas_price, gas_price);
    assert_eq!(estimated_transaction_cost.gas_used, expected_gas_used);
    assert_eq!(
        estimated_transaction_cost.metered_bytes_size,
        expected_metered_bytes_size
    );
    assert_eq!(estimated_transaction_cost.total_fee, expected_total_fee);
    Ok(())
}

#[tokio::test]
async fn contract_call_has_same_estimated_and_used_gas() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let tolerance = 0.0;
    let contract_methods = contract_instance.methods();
    let estimated_gas_used = contract_methods
        .initialize_counter(42) // Build the ABI call
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?
        .gas_used;

    let gas_used = contract_methods
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?
        .gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn mutl_call_has_same_estimated_and_used_gas() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.initialize_counter(42);
    let call_handler_2 = contract_methods.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let tolerance = 0.0;
    let estimated_gas_used = multi_call_handler
        .estimate_transaction_cost(Some(tolerance)) // Perform the network call
        .await?
        .gas_used;

    let gas_used = multi_call_handler.call::<(u64, [u64; 2])>().await?.gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn contract_method_call_respects_maturity() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/transaction_block_height"
    );

    let call_w_maturity = |call_maturity| {
        let mut prepared_call = contract_instance
            .methods()
            .calling_this_will_produce_a_block();
        prepared_call.tx_parameters.maturity = call_maturity;
        prepared_call.call()
    };

    call_w_maturity(1).await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");
    Ok(())
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/auth_testing_contract"
    );

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = contract_instance
        .methods()
        .check_msg_sender(wallet.address().into())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}

//-----TODO: sort
#[tokio::test]
async fn test_amount_and_asset_forwarding() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/token_ops"
    );
    let contract_id = contract_instance.get_contract_id();
    let contract_methods = contract_instance.methods();

    let mut balance_response = contract_methods
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 0);

    contract_methods.mint_coins(5_000_000).call().await?;

    balance_response = contract_methods
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = contract_methods
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    contract_methods
        .transfer_coins_to_output(1_000_000, contract_id.into(), address.into())
        .append_variable_outputs(1)
        .call()
        .await?;

    let asset_id = AssetId::from(*contract_id.hash());
    let call_params = CallParameters::new(Some(0), Some(asset_id), None);
    let tx_params = TxParameters::new(None, Some(1_000_000), None);

    let response = contract_methods
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*contract_id.hash())
    );
    Ok(())
}

#[tokio::test]
async fn test_get_gas_used() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/contract_test"
    );

    let gas_used = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?
        .gas_used;

    assert!(gas_used > 0);
    Ok(())
}

#[tokio::test]
async fn test_large_return_data() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/behaviors/large_return_data"
    );

    let contract_methods = contract_instance.methods();
    let res = contract_methods.get_id().call().await?;

    assert_eq!(
        res.value.0,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );

    // One word-sized string
    let res = contract_methods.get_small_string().call().await?;
    assert_eq!(res.value, "gggggggg");

    // Two word-sized string
    let res = contract_methods.get_large_string().call().await?;
    assert_eq!(res.value, "ggggggggg");

    // Large struct will be bigger than a `WORD`.
    let res = contract_methods.get_large_struct().call().await?;
    assert_eq!(res.value.foo, 12);
    assert_eq!(res.value.bar, 42);

    // Array will be returned in `ReturnData`.
    let res = contract_methods.get_large_array().call().await?;
    assert_eq!(res.value, [1, 2]);

    let res = contract_methods.get_contract_id().call().await?;

    // First `value` is from `CallResponse`.
    // Second `value` is from the `ContractId` type.
    assert_eq!(
        res.value,
        ContractId::from([
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ])
    );
    Ok(())
}
