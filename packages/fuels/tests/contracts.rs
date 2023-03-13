#[allow(unused_imports)]
use std::future::Future;

use fuels::prelude::*;
use fuels_types::Bits256;

#[tokio::test]
async fn test_multiple_args() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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
async fn test_contract_calling_contract() -> Result<()> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "LibContract",
            abi = "packages/fuels/tests/contracts/lib_contract"
        ),
        Abigen(
            name = "LibContractCaller",
            abi = "packages/fuels/tests/contracts/lib_contract_caller"
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "lib_contract_instance2",
            contract = "LibContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet"
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();
    let lib_contract_id2 = lib_contract_instance2.contract_id();

    // Call the contract directly. It increments the given value.
    let response = lib_contract_instance.methods().increment(42).call().await?;

    assert_eq!(43, response.value);

    let response = contract_caller_instance
        .methods()
        .increment_from_contracts(lib_contract_id.into(), lib_contract_id2.into(), 42)
        // Note that the two lib_contract_instances have different types
        .set_contracts(&[&lib_contract_instance, &lib_contract_instance2])
        .call()
        .await?;

    assert_eq!(86, response.value);

    // ANCHOR: external_contract
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id.into(), 42)
        .set_contracts(&[&lib_contract_instance])
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert_eq!(43, response.value);

    // ANCHOR: external_contract_ids
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id.into(), 42)
        .set_contract_ids(&[lib_contract_id.clone()])
        .call()
        .await?;
    // ANCHOR_END: external_contract_ids

    assert_eq!(43, response.value);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "RevertContract",
            abi = "packages/fuels/tests/contracts/revert_transaction_error"
        ),
        Deploy(
            name = "contract_instance",
            contract = "RevertContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance
        .methods()
        .make_transaction_fail(0)
        .call()
        .await;

    assert!(matches!(
        response,
        Err(Error::RevertTransactionError { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn test_multiple_read_calls() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "MultiReadContract",
            abi = "packages/fuels/tests/contracts/multiple_read_calls"
        ),
        Deploy(
            name = "contract_instance",
            contract = "MultiReadContract",
            wallet = "wallet"
        ),
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
async fn test_multi_call() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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
async fn test_contract_call_fee_estimation() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let gas_price = 100_000_000;
    let gas_limit = 800;
    let tolerance = 0.2;

    let expected_min_gas_price = 0; // This is the default min_gas_price from the ConsensusParameters
    let expected_gas_used = 516;
    let expected_metered_bytes_size = 720;
    let expected_total_fee = 368;

    let estimated_transaction_cost = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(
            TxParameters::default()
                .set_gas_price(gas_price)
                .set_gas_limit(gas_limit),
        )
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
async fn contract_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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
async fn mutl_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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
async fn contract_method_call_respects_maturity() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "BlockHeightContract",
            abi = "packages/fuels/tests/contracts/transaction_block_height"
        ),
        Deploy(
            name = "contract_instance",
            contract = "BlockHeightContract",
            wallet = "wallet"
        ),
    );

    let call_w_maturity = |maturity| {
        contract_instance
            .methods()
            .calling_this_will_produce_a_block()
            .tx_params(TxParameters::default().set_maturity(maturity))
    };

    call_w_maturity(1).call().await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).call().await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");
    Ok(())
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "AuthContract",
            abi = "packages/fuels/tests/contracts/auth_testing_contract"
        ),
        Deploy(
            name = "contract_instance",
            contract = "AuthContract",
            wallet = "wallet"
        ),
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
async fn test_large_return_data() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/large_return_data"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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

    // First `value` is from `FuelCallResponse`.
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
async fn can_handle_function_called_new() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance.methods().new().call().await?.value;

    assert_eq!(response, 12345);
    Ok(())
}

#[tokio::test]
async fn test_contract_setup_macro_deploy_with_salt() -> Result<()> {
    // ANCHOR: contract_setup_macro_multi
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "LibContract",
            abi = "packages/fuels/tests/contracts/lib_contract"
        ),
        Abigen(
            name = "LibContractCaller",
            abi = "packages/fuels/tests/contracts/lib_contract_caller"
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance2",
            contract = "LibContractCaller",
            wallet = "wallet"
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();

    let contract_caller_id = contract_caller_instance.contract_id();

    let contract_caller_id2 = contract_caller_instance2.contract_id();

    // Because we deploy with salt, we can deploy the same contract multiple times
    assert_ne!(contract_caller_id, contract_caller_id2);

    // The first contract can be called because they were deployed on the same provider
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id.into(), 42)
        .set_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);

    let response = contract_caller_instance2
        .methods()
        .increment_from_contract(lib_contract_id.into(), 42)
        .set_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    assert_eq!(contract_instance.wallet().address(), wallet.address());
    //`contract_id()` is tested in
    // async fn test_contract_calling_contract() -> Result<()> {
    Ok(())
}

#[tokio::test]
async fn test_connect_wallet() -> Result<()> {
    // ANCHOR: contract_setup_macro_manual_wallet
    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
    let wallet = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_contract_test!(
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );
    // ANCHOR_END: contract_setup_macro_manual_wallet

    // pay for call with wallet
    let tx_params = TxParameters::default()
        .set_gas_price(10)
        .set_gas_limit(10000);
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
) -> Result<(Vec<WalletUnlocked>, [Address; 3], AssetId, Bech32ContractId)> {
    let wallet_config = WalletsConfig::new(Some(3), None, None);
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await;

    let contract_id = Contract::deploy(
        "tests/contracts/token_ops/out/debug/token_ops.bin",
        &wallets[0],
        DeployConfiguration::default(),
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
async fn test_output_variable_estimation() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    ));

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

        assert!(matches!(
            response,
            Err(Error::RevertTransactionError { .. })
        ));
    }

    {
        // Should fail due to insufficient attempts (needs at least 3)
        let response = contract_methods
            .mint_to_addresses(amount, addresses)
            .estimate_tx_dependencies(Some(2))
            .await;

        assert!(matches!(
            response,
            Err(Error::RevertTransactionError { .. })
        ));
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
async fn test_output_message_estimation() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    ));

    let (wallets, _, _, contract_id) = setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    let amount = 1000;

    let address = Bits256([1u8; 32]);
    {
        // Should fail due to lack of output messages
        let response = contract_methods.send_message(address, amount).call().await;

        assert!(matches!(
            response,
            Err(Error::RevertTransactionError { .. })
        ));
    }

    {
        // Should add 1 output message automatically
        let _ = contract_methods
            .send_message(address, amount)
            .estimate_tx_dependencies(Some(1))
            .await?
            .call()
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation_default_attempts() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    ));

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
async fn test_output_variable_estimation_multicall() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
    ));

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

    let base_layer_addres = Bits256([1u8; 32]);
    let call_handler = contract_methods.send_message(base_layer_addres, amount);
    multi_call_handler.add_call(call_handler);

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
async fn test_contract_instance_get_balances() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);
    let (coins, asset_ids) = setup_multiple_assets_coins(wallet.address(), 2, 4, 8);
    let random_asset_id = &asset_ids[1];
    let (provider, _) = setup_test_provider(coins.clone(), vec![], None, None).await;
    wallet.set_provider(provider.clone());

    setup_contract_test!(
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );
    let contract_id = contract_instance.contract_id();

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

    let random_asset_id_key = format!("{random_asset_id:#x}");
    let random_asset_balance = contract_balances.get(&random_asset_id_key).unwrap();
    assert_eq!(*random_asset_balance, amount);

    Ok(())
}

#[tokio::test]
async fn contract_call_futures_implement_send() -> Result<()> {
    fn tokio_spawn_imitation<T>(_: T)
    where
        T: Future + Send + 'static,
    {
    }

    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
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
async fn test_contract_set_estimation() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "LibContract",
            abi = "packages/fuels/tests/contracts/lib_contract"
        ),
        Abigen(
            name = "LibContractCaller",
            abi = "packages/fuels/tests/contracts/lib_contract_caller"
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet"
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();

    let res = lib_contract_instance.methods().increment(42).call().await?;
    assert_eq!(43, res.value);

    {
        // Should fail due to missing external contracts
        let res = contract_caller_instance
            .methods()
            .increment_from_contract(lib_contract_id.into(), 42)
            .call()
            .await;

        assert!(matches!(res, Err(Error::RevertTransactionError { .. })));
    }

    let res = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id.into(), 42)
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    assert_eq!(43, res.value);
    Ok(())
}

#[tokio::test]
async fn test_output_variable_contract_id_estimation_multicall() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "LibContract",
            abi = "packages/fuels/tests/contracts/lib_contract"
        ),
        Abigen(
            name = "LibContractCaller",
            abi = "packages/fuels/tests/contracts/lib_contract_caller"
        ),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_test_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let lib_contract_id = lib_contract_instance.contract_id();

    let contract_methods = contract_caller_instance.methods();

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());
    multi_call_handler.tx_params(Default::default());

    (0..3).for_each(|_| {
        let call_handler = contract_methods.increment_from_contract(lib_contract_id.into(), 42);
        multi_call_handler.add_call(call_handler);
    });

    // add call that does not need ContractId
    let contract_methods = contract_test_instance.methods();
    let call_handler = contract_methods.get(5, 6);

    multi_call_handler.add_call(call_handler);

    let call_response = multi_call_handler
        .estimate_tx_dependencies(None)
        .await?
        .call::<(u64, u64, u64, u64)>()
        .await?;

    assert_eq!(call_response.value, (43, 43, 43, 5));

    Ok(())
}

#[tokio::test]
async fn test_contract_call_with_non_default_max_input() -> Result<()> {
    use fuels::tx::ConsensusParameters;
    use fuels_types::coin::Coin;

    let consensus_parameters_config = ConsensusParameters::DEFAULT.with_max_inputs(123);

    let mut wallet = WalletUnlocked::new_random(None);

    let coins: Vec<Coin> = setup_single_asset_coins(
        wallet.address(),
        Default::default(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );

    let (fuel_client, _) =
        setup_test_client(coins, vec![], None, None, Some(consensus_parameters_config)).await;
    let provider = Provider::new(fuel_client);
    wallet.set_provider(provider.clone());

    setup_contract_test!(
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance.methods().get(5, 6).call().await?;

    assert_eq!(response.value, 5);

    Ok(())
}

#[tokio::test]
async fn test_add_custom_assets() -> Result<()> {
    let initial_amount = 100_000;
    let asset_base = AssetConfig {
        id: BASE_ASSET_ID,
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let asset_id_1 = AssetId::from([3u8; 32]);
    let asset_1 = AssetConfig {
        id: asset_id_1,
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let asset_id_2 = AssetId::from([1u8; 32]);
    let asset_2 = AssetConfig {
        id: asset_id_2,
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let assets = vec![asset_base, asset_1, asset_2];

    let num_wallets = 2;
    let wallet_config = WalletsConfig::new_multiple_assets(num_wallets, assets);
    let mut wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await;
    let wallet_1 = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_contract_test!(
        Abigen(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test"
        ),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet_1"
        ),
    );

    let amount_1 = 5000;
    let amount_2 = 3000;
    let response = contract_instance
        .methods()
        .get(5, 6)
        .add_custom_asset(asset_id_1, amount_1, Some(wallet_2.address().clone()))
        .add_custom_asset(asset_id_2, amount_2, Some(wallet_2.address().clone()))
        .call()
        .await?;

    assert_eq!(response.value, 5);

    let balance_asset_1 = wallet_1.get_asset_balance(&asset_id_1).await?;
    let balance_asset_2 = wallet_1.get_asset_balance(&asset_id_2).await?;
    assert_eq!(balance_asset_1, initial_amount - amount_1);
    assert_eq!(balance_asset_2, initial_amount - amount_2);

    let balance_asset_1 = wallet_2.get_asset_balance(&asset_id_1).await?;
    let balance_asset_2 = wallet_2.get_asset_balance(&asset_id_2).await?;
    assert_eq!(balance_asset_1, initial_amount + amount_1);
    assert_eq!(balance_asset_2, initial_amount + amount_2);

    Ok(())
}

#[tokio::test]
async fn test_contract_raw_slice() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await;
    setup_contract_test!(
        Abigen(
            name = "RawSliceContract",
            abi = "packages/fuels/tests/contracts/contract_raw_slice"
        ),
        Deploy(
            name = "contract_instance",
            contract = "RawSliceContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    for length in 0..=10 {
        let response = contract_methods.return_raw_slice(length).call().await?;
        assert_eq!(response.value, (0..length).collect::<Vec<_>>());
    }

    Ok(())
}

#[tokio::test]
async fn test_deploy_error_messages() {
    let wallet = launch_provider_and_get_wallet().await;
    {
        let binary_path =
            "../../packages/fuels/tests/contracts/contract_test/out/debug/no_file_on_path.bin";
        let expected = format!("Invalid data: file '{binary_path}' does not exist");

        let response = Contract::deploy(binary_path, &wallet, DeployConfiguration::default())
            .await
            .expect_err("Should have failed");

        assert_eq!(response.to_string(), expected);
    }
    {
        let binary_path =
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json";
        let expected = format!("Invalid data: expected `{binary_path}` to have '.bin' extension");

        let response = Contract::deploy(binary_path, &wallet, DeployConfiguration::default())
            .await
            .expect_err("Should have failed");

        assert_eq!(response.to_string(), expected);
    }
}

#[tokio::test]
async fn test_payable_annotation() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TestContract",
            abi = "packages/fuels/tests/contracts/payable_annotation"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    let response = contract_methods
        .payable()
        .call_params(
            CallParameters::default()
                .set_amount(100)
                .set_gas_forwarded(20_000),
        )?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    // ANCHOR: non_payable_params
    let err = contract_methods
        .non_payable()
        .call_params(CallParameters::default().set_amount(100))
        .expect_err("Should return call params error.");

    assert!(matches!(err, Error::AssetsForwardedToNonPayableMethod));
    // ANCHOR_END: non_payable_params

    let response = contract_methods
        .non_payable()
        .call_params(CallParameters::default().set_gas_forwarded(20_000))?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    Ok(())
}
