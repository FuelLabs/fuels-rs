use fuels::prelude::*;
use std::future::Future;

#[tokio::test]
async fn test_multiple_args() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
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
        "packages/fuels/tests/contracts/foo_contract"
    );
    let foo_contract_id = foo_contract_instance.get_contract_id();

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.methods().foo(true).call().await?;
    assert!(!res.value);

    // Load and deploy the second compiled contract
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/contracts/foo_caller_contract"
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

    assert!(res.value);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/revert_transaction_error"
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
        "packages/fuels/tests/contracts/multiple_read_calls"
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
async fn test_multi_call() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
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
async fn test_contract_call_fee_estimation() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
    );

    let gas_price = 100_000_000;
    let gas_limit = 800;
    let tolerance = 0.2;

    let expected_min_gas_price = 0; // This is the default min_gas_price from the ConsensusParameters
    let expected_gas_used = 3474;
    let expected_metered_bytes_size = 720;
    let expected_total_fee = 636;

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
        "packages/fuels/tests/contracts/contract_test"
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
        "packages/fuels/tests/contracts/contract_test"
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
        "packages/fuels/tests/contracts/transaction_block_height"
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
        "packages/fuels/tests/contracts/auth_testing_contract"
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

#[tokio::test]
async fn test_large_return_data() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/large_return_data"
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

#[tokio::test]
async fn can_handle_function_called_new() -> anyhow::Result<()> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
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
        "packages/fuels/tests/contracts/foo_contract"
    );
    let foo_contract_id = foo_contract_instance.get_contract_id();

    // The macros that want to use the `wallet` have to set
    // the wallet name to `None`
    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/contracts/foo_caller_contract"
    );
    let foo_caller_contract_id = foo_caller_contract_instance.get_contract_id();

    setup_contract_test!(
        foo_caller_contract_instance2,
        None,
        "packages/fuels/tests/contracts/foo_caller_contract"
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
    assert!(res.value);

    let res = foo_caller_contract_instance2
        .methods()
        .call_foo_contract(Bits256(bits), true)
        .set_contracts(&[foo_contract_id.clone()]) // Sets the external contract
        .call()
        .await?;
    assert!(res.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
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

    let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
    let wallet = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_contract_test!(
        contract_instance,
        None,
        "packages/fuels/tests/contracts/contract_test"
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

async fn setup_output_variable_estimation_test(
) -> Result<(Vec<WalletUnlocked>, [Address; 3], AssetId, Bech32ContractId), Error> {
    let wallet_config = WalletsConfig::new(Some(3), None, None);
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await;

    let contract_id = Contract::deploy(
        "tests/contracts/token_ops/out/debug/token_ops.bin",
        &wallets[0],
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let mint_asset_id = AssetId::from(*contract_id.hash());
    let addresses: [Address; 3] = wallets
        .iter()
        .map(|wallet| wallet.address().into())
        .collect::<Vec<Address>>()
        .try_into()
        .unwrap();

    Ok((wallets, addresses, mint_asset_id, contract_id))
}

#[tokio::test]
async fn test_output_variable_estimation() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    );

    let (wallets, addresses, mint_asset_id, contract_id) =
        setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    let amount = 1000;

    {
        // Should fail due to lack of output variables
        let response = contract_methods
            .mint_to_addresses(amount, addresses)
            .call()
            .await;

        assert!(matches!(response, Err(Error::RevertTransactionError(..))));
    }

    {
        // Should fail due to insufficient attempts (needs at least 3)
        let response = contract_methods
            .mint_to_addresses(amount, addresses)
            .estimate_tx_dependencies(Some(2))
            .await;

        assert!(matches!(response, Err(Error::RevertTransactionError(..))));
    }

    {
        // Should add 3 output variables automatically
        let _ = contract_methods
            .mint_to_addresses(amount, addresses)
            .estimate_tx_dependencies(Some(3))
            .await?
            .call()
            .await?;

        for wallet in wallets.iter() {
            let balance = wallet.get_asset_balance(&mint_asset_id).await?;
            assert_eq!(balance, amount);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation_default_attempts() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    );

    let (wallets, addresses, mint_asset_id, contract_id) =
        setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    let amount = 1000;

    let _ = contract_methods
        .mint_to_addresses(amount, addresses)
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    for wallet in wallets.iter() {
        let balance = wallet.get_asset_balance(&mint_asset_id).await?;
        assert_eq!(balance, amount);
    }

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation_multicall() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    );

    let (wallets, addresses, mint_asset_id, contract_id) =
        setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    let amount = 1000;

    let mut multi_call_handler = MultiContractCallHandler::new(wallets[0].clone());
    (0..3).for_each(|_| {
        let call_handler = contract_methods.mint_to_addresses(amount, addresses);
        multi_call_handler.add_call(call_handler);
    });

    let _ = multi_call_handler
        .estimate_tx_dependencies(None)
        .await?
        .call::<((), (), ())>()
        .await?;

    for wallet in wallets.iter() {
        let balance = wallet.get_asset_balance(&mint_asset_id).await?;
        assert_eq!(balance, 3 * amount);
    }

    Ok(())
}

#[tokio::test]
async fn test_contract_instance_get_balances() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let (coins, asset_ids) = setup_multiple_assets_coins(wallet.address(), 2, 4, 8);
    let random_asset_id = &asset_ids[1];
    let (provider, _) = setup_test_provider(coins.clone(), vec![], None, None).await;
    wallet.set_provider(provider.clone());

    setup_contract_test!(
        contract_instance,
        None,
        "packages/fuels/tests/contracts/contract_test"
    );
    let contract_id = contract_instance.get_contract_id();

    // Check the current balance of the contract with id 'contract_id'
    let contract_balances = contract_instance.get_balances().await?;
    assert!(contract_balances.is_empty());

    // Transfer an amount to the contract
    let amount = 8;
    let _receipts = wallet
        .force_transfer_to_contract(
            contract_id,
            amount,
            *random_asset_id,
            TxParameters::default(),
        )
        .await?;

    // Check that the contract now has 1 coin
    let contract_balances = contract_instance.get_balances().await?;
    assert_eq!(contract_balances.len(), 1);

    let random_asset_id_key = format!("{:#x}", random_asset_id);
    let random_asset_balance = contract_balances.get(&random_asset_id_key).unwrap();
    assert_eq!(*random_asset_balance, amount);

    Ok(())
}

#[tokio::test]
async fn contract_call_futures_implement_send() -> Result<(), Error> {
    fn tokio_spawn_imitation<T>(_: T)
    where
        T: Future + Send + 'static,
    {
    }

    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
    );

    tokio_spawn_imitation(async move {
        contract_instance
            .methods()
            .initialize_counter(42)
            .call()
            .await
            .unwrap();
    });

    Ok(())
}

#[tokio::test]
async fn test_contract_set_estimation() -> Result<(), Error> {
    setup_contract_test!(
        foo_contract_instance,
        wallet,
        "packages/fuels/tests/contracts/foo_contract"
    );
    let foo_contract_id = foo_contract_instance.get_contract_id();

    let res = foo_contract_instance.methods().foo(true).call().await?;
    assert!(!res.value);

    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/contracts/foo_caller_contract"
    );

    let bits = *foo_contract_id.hash();

    {
        // Should fail due to missing external contracts
        let res = foo_caller_contract_instance
            .methods()
            .call_foo_contract(Bits256(bits), true)
            .call()
            .await;
        assert!(matches!(res, Err(Error::RevertTransactionError(..))));
    }

    let res = foo_caller_contract_instance
        .methods()
        .call_foo_contract(Bits256(bits), true)
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    assert!(res.value);
    Ok(())
}

#[tokio::test]
async fn test_output_variable_contract_id_estimation_multicall() -> Result<(), Error> {
    setup_contract_test!(
        foo_contract_instance,
        wallet,
        "packages/fuels/tests/contracts/foo_contract"
    );

    let foo_contract_id = foo_contract_instance.get_contract_id();

    setup_contract_test!(
        foo_caller_contract_instance,
        None,
        "packages/fuels/tests/contracts/foo_caller_contract"
    );

    setup_contract_test!(
        contract_test_instance,
        None,
        "packages/fuels/tests/contracts/contract_test"
    );

    let bits = *foo_contract_id.hash();
    let contract_methods = foo_caller_contract_instance.methods();

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());
    multi_call_handler.tx_params(Default::default());

    (0..3).for_each(|_| {
        let call_handler = contract_methods.call_foo_contract(Bits256(bits), true);
        multi_call_handler.add_call(call_handler);
    });

    // add call that does not need ContractId
    let contract_methods = contract_test_instance.methods();
    let call_handler = contract_methods.get(5, 6);

    multi_call_handler.add_call(call_handler);

    let call_response = multi_call_handler
        .estimate_tx_dependencies(None)
        .await?
        .call::<(bool, bool, bool, u64)>()
        .await?;

    assert_eq!(call_response.value, (true, true, true, 5));

    Ok(())
}
