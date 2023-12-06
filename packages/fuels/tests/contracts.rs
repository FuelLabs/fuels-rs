#[allow(unused_imports)]
use std::future::Future;
use std::vec;

use fuels::{
    accounts::{predicate::Predicate, Account},
    core::codec::{calldata, fn_selector},
    prelude::*,
    types::Bits256,
};
use fuels_core::codec::DecoderConfig;

#[tokio::test]
async fn test_multiple_args() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    // Make sure we can call the contract with multiple arguments
    let contract_methods = contract_instance.methods();
    let response = contract_methods.get(5, 6).call().await?;

    assert_eq!(response.value, 11);

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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "packages/fuels/tests/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "packages/fuels/tests/contracts/lib_contract_caller"
            ),
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
        .increment_from_contracts(lib_contract_id, lib_contract_id2, 42)
        // Note that the two lib_contract_instances have different types
        .with_contracts(&[&lib_contract_instance, &lib_contract_instance2])
        .call()
        .await?;

    assert_eq!(86, response.value);

    // ANCHOR: external_contract
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert_eq!(43, response.value);

    // ANCHOR: external_contract_ids
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contract_ids(&[lib_contract_id.clone()])
        .call()
        .await?;
    // ANCHOR_END: external_contract_ids

    assert_eq!(43, response.value);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RevertContract",
            project = "packages/fuels/tests/contracts/revert_transaction_error"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RevertContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance
        .methods()
        .make_transaction_fail(true)
        .call()
        .await;

    assert!(matches!(
        response,
        Err(Error::RevertTransactionError { revert_id, .. }) if revert_id == 128
    ));

    Ok(())
}

#[tokio::test]
async fn test_multiple_read_calls() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MultiReadContract",
            project = "packages/fuels/tests/contracts/multiple_read_calls"
        )),
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
    let stored = contract_methods.read().simulate().await?;

    assert_eq!(stored.value, 42);

    let stored = contract_methods.read().simulate().await?;

    assert_eq!(stored.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_multi_call_beginner() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(7);
    let call_handler_2 = contract_methods.get_single(42);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let (val_1, val_2): (u64, u64) = multi_call_handler.call().await?.value;

    assert_eq!(val_1, 7);
    assert_eq!(val_2, 42);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_pro() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let my_type_1 = MyType { x: 1, y: 2 };
    let my_type_2 = MyType { x: 3, y: 4 };

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(5);
    let call_handler_2 = contract_methods.get_single(6);
    let call_handler_3 = contract_methods.get_alt(my_type_1.clone());
    let call_handler_4 = contract_methods.get_alt(my_type_2.clone());
    let call_handler_5 = contract_methods.get_array([7; 2]);
    let call_handler_6 = contract_methods.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2)
        .add_call(call_handler_3)
        .add_call(call_handler_4)
        .add_call(call_handler_5)
        .add_call(call_handler_6);

    let (val_1, val_2, type_1, type_2, array_1, array_2): (
        u64,
        u64,
        MyType,
        MyType,
        [u64; 2],
        [u64; 2],
    ) = multi_call_handler.call().await?.value;

    assert_eq!(val_1, 5);
    assert_eq!(val_2, 6);
    assert_eq!(type_1, my_type_1);
    assert_eq!(type_2, my_type_2);
    assert_eq!(array_1, [7; 2]);
    assert_eq!(array_2, [42; 2]);

    Ok(())
}

#[tokio::test]
async fn test_contract_call_fee_estimation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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
    let expected_gas_used = 675;
    let expected_metered_bytes_size = 792;
    let expected_total_fee = 898;

    let estimated_transaction_cost = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(
            TxPolicies::default()
                .with_gas_price(gas_price)
                .with_script_gas_limit(gas_limit),
        )
        .estimate_transaction_cost(Some(tolerance))
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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let tolerance = 0.0;
    let contract_methods = contract_instance.methods();
    let estimated_gas_used = contract_methods
        .initialize_counter(42)
        .estimate_transaction_cost(Some(tolerance))
        .await?
        .gas_used;

    let gas_used = contract_methods
        .initialize_counter(42)
        .call()
        .await?
        .gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn mult_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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
        .estimate_transaction_cost(Some(tolerance))
        .await?
        .gas_used;

    let gas_used = multi_call_handler.call::<(u64, [u64; 2])>().await?.gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn contract_method_call_respects_maturity() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "BlockHeightContract",
            project = "packages/fuels/tests/contracts/transaction_block_height"
        )),
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
            .with_tx_policies(TxPolicies::default().with_maturity(maturity))
    };

    call_w_maturity(1).call().await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).call().await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");
    Ok(())
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "AuthContract",
            project = "packages/fuels/tests/contracts/auth_testing_contract"
        )),
        Deploy(
            name = "contract_instance",
            contract = "AuthContract",
            wallet = "wallet"
        ),
    );

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = contract_instance
        .methods()
        .check_msg_sender(wallet.address())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}

#[tokio::test]
async fn test_large_return_data() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/large_return_data"
        )),
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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "packages/fuels/tests/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "packages/fuels/tests/contracts/lib_contract_caller"
            ),
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
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);

    let response = contract_caller_instance2
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    assert_eq!(contract_instance.account().address(), wallet.address());
    //`contract_id()` is tested in
    // async fn test_contract_calling_contract() -> Result<()> {
    Ok(())
}

#[tokio::test]
async fn test_connect_wallet() -> Result<()> {
    // ANCHOR: contract_setup_macro_manual_wallet
    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;
    let wallet = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );
    // ANCHOR_END: contract_setup_macro_manual_wallet

    // pay for call with wallet
    let tx_policies = TxPolicies::default()
        .with_gas_price(10)
        .with_script_gas_limit(1_000_000);
    contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    // confirm that funds have been deducted
    let wallet_balance = wallet.get_asset_balance(&Default::default()).await?;
    assert!(DEFAULT_COIN_AMOUNT > wallet_balance);

    // pay for call with wallet_2
    contract_instance
        .with_account(wallet_2.clone())?
        .methods()
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
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
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;

    let contract_id = Contract::load_from(
        "tests/contracts/token_ops/out/debug/token_ops.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallets[0], TxPolicies::default())
    .await?;

    let mint_asset_id = contract_id.asset_id(&Bits256::zeroed());
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

    let contract_instance = MyContract::new(contract_id.clone(), wallets[0].clone());
    let contract_methods = contract_instance.methods();
    const NUM_OF_CALLS: u64 = 3;
    let amount = 1000;
    let total_amount = amount * NUM_OF_CALLS;

    let mut multi_call_handler = MultiContractCallHandler::new(wallets[0].clone());
    (0..NUM_OF_CALLS).for_each(|_| {
        let call_handler = contract_methods.mint_to_addresses(amount, addresses);
        multi_call_handler.add_call(call_handler);
    });

    wallets[0]
        .force_transfer_to_contract(
            &contract_id,
            total_amount,
            AssetId::BASE,
            TxPolicies::default(),
        )
        .await
        .unwrap();

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
    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    wallet.set_provider(provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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
    wallet
        .force_transfer_to_contract(contract_id, amount, *random_asset_id, TxPolicies::default())
        .await?;

    // Check that the contract now has 1 coin
    let contract_balances = contract_instance.get_balances().await?;
    assert_eq!(contract_balances.len(), 1);

    let random_asset_balance = contract_balances.get(random_asset_id).unwrap();
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

    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "packages/fuels/tests/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "packages/fuels/tests/contracts/lib_contract_caller"
            ),
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
            .increment_from_contract(lib_contract_id, 42)
            .call()
            .await;

        assert!(matches!(res, Err(Error::RevertTransactionError { .. })));
    }

    let res = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    assert_eq!(43, res.value);
    Ok(())
}

#[tokio::test]
async fn test_output_variable_contract_id_estimation_multicall() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "packages/fuels/tests/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "packages/fuels/tests/contracts/lib_contract_caller"
            ),
            Contract(
                name = "TestContract",
                project = "packages/fuels/tests/contracts/contract_test"
            ),
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

    let mut multi_call_handler =
        MultiContractCallHandler::new(wallet.clone()).with_tx_policies(Default::default());

    (0..3).for_each(|_| {
        let call_handler = contract_methods.increment_from_contract(lib_contract_id, 42);
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

    assert_eq!(call_response.value, (43, 43, 43, 11));

    Ok(())
}

#[tokio::test]
async fn test_contract_call_with_non_default_max_input() -> Result<()> {
    use fuels::{
        tx::{ConsensusParameters, TxParameters},
        types::coin::Coin,
    };

    let consensus_parameters = ConsensusParameters {
        tx_params: TxParameters::default().with_max_inputs(123),
        ..Default::default()
    };

    let mut wallet = WalletUnlocked::new_random(None);

    let coins: Vec<Coin> = setup_single_asset_coins(
        wallet.address(),
        Default::default(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let chain_config = ChainConfig {
        consensus_parameters: consensus_parameters.clone(),
        ..ChainConfig::default()
    };

    let provider = setup_test_provider(coins, vec![], None, Some(chain_config)).await?;
    wallet.set_provider(provider.clone());
    assert_eq!(consensus_parameters, *provider.consensus_parameters());

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let response = contract_instance.methods().get(5, 6).call().await?;

    assert_eq!(response.value, 11);

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
    let mut wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;
    let wallet_1 = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
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

    assert_eq!(response.value, 11);

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
async fn contract_load_error_messages() {
    {
        let binary_path = "tests/contracts/contract_test/out/debug/no_file_on_path.bin";
        let expected_error = format!("Invalid data: file \"{binary_path}\" does not exist");

        let error = Contract::load_from(binary_path, LoadConfiguration::default())
            .expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
    {
        let binary_path = "tests/contracts/contract_test/out/debug/contract_test-abi.json";
        let expected_error =
            format!("Invalid data: expected \"{binary_path}\" to have '.bin' extension");

        let error = Contract::load_from(binary_path, LoadConfiguration::default())
            .expect_err("Should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
}

#[tokio::test]
async fn test_payable_annotation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/payable_annotation"
        )),
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
                .with_amount(100)
                .with_gas_forwarded(20_000),
        )?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    // ANCHOR: non_payable_params
    let err = contract_methods
        .non_payable()
        .call_params(CallParameters::default().with_amount(100))
        .expect_err("Should return call params error.");

    assert!(matches!(err, Error::AssetsForwardedToNonPayableMethod));
    // ANCHOR_END: non_payable_params

    let response = contract_methods
        .non_payable()
        .call_params(CallParameters::default().with_gas_forwarded(20_000))?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    Ok(())
}

#[tokio::test]
async fn multi_call_from_calls_with_different_account_types() -> Result<()> {
    use fuels::prelude::*;

    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let wallet = WalletUnlocked::new_random(None);
    let predicate = Predicate::from_code(vec![]);

    let contract_methods_wallet =
        MyContract::new(Bech32ContractId::default(), wallet.clone()).methods();
    let contract_methods_predicate =
        MyContract::new(Bech32ContractId::default(), predicate).methods();

    let call_handler_1 = contract_methods_wallet.initialize_counter(42);
    let call_handler_2 = contract_methods_predicate.get_array([42; 2]);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet);

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    Ok(())
}

#[tokio::test]
async fn low_level_call() -> Result<()> {
    use fuels::types::SizedAsciiString;

    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "MyCallerContract",
                project = "packages/fuels/tests/contracts/low_level_caller"
            ),
            Contract(
                name = "MyTargetContract",
                project = "packages/fuels/tests/contracts/contract_test"
            ),
        ),
        Deploy(
            name = "caller_contract_instance",
            contract = "MyCallerContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "target_contract_instance",
            contract = "MyTargetContract",
            wallet = "wallet"
        ),
    );

    let function_selector = fn_selector!(initialize_counter(u64));
    let call_data = calldata!(42u64)?;

    caller_contract_instance
        .methods()
        .call_low_level_call(
            target_contract_instance.id(),
            Bytes(function_selector),
            Bytes(call_data),
            true,
        )
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    let response = target_contract_instance
        .methods()
        .get_counter()
        .call()
        .await?;
    assert_eq!(response.value, 42);

    let function_selector =
        fn_selector!(set_value_multiple_complex(MyStruct, SizedAsciiString::<4>));
    let call_data = calldata!(
        MyStruct {
            a: true,
            b: [1, 2, 3],
        },
        SizedAsciiString::<4>::try_from("fuel")?
    )?;

    caller_contract_instance
        .methods()
        .call_low_level_call(
            target_contract_instance.id(),
            Bytes(function_selector),
            Bytes(call_data),
            false,
        )
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    let result_uint = target_contract_instance
        .methods()
        .get_counter()
        .call()
        .await
        .unwrap()
        .value;

    let result_bool = target_contract_instance
        .methods()
        .get_bool_value()
        .call()
        .await
        .unwrap()
        .value;

    let result_str = target_contract_instance
        .methods()
        .get_str_value()
        .call()
        .await
        .unwrap()
        .value;

    assert_eq!(result_uint, 42);
    assert!(result_bool);
    assert_eq!(result_str, "fuel");

    Ok(())
}

#[cfg(any(not(feature = "fuel-core-lib"), feature = "rocksdb"))]
#[test]
fn db_rocksdb() {
    use std::{fs, str::FromStr};

    use fuels::{
        accounts::{fuel_crypto::SecretKey, wallet::WalletUnlocked},
        client::{PageDirection, PaginationRequest},
        prelude::{setup_test_provider, DbType, ViewOnlyAccount, DEFAULT_COIN_AMOUNT},
    };

    let temp_dir = tempfile::tempdir()
        .expect("Failed to make tempdir")
        .into_path();
    let temp_dir_name = temp_dir
        .file_name()
        .expect("Failed to get file name")
        .to_string_lossy()
        .to_string();
    let temp_database_path = temp_dir.join("db");

    tokio::runtime::Runtime::new()
        .expect("Tokio runtime failed")
        .block_on(async {
            let wallet = WalletUnlocked::new_from_private_key(
                SecretKey::from_str(
                    "0x4433d156e8c53bf5b50af07aa95a29436f29a94e0ccc5d58df8e57bdc8583c32",
                )
                .unwrap(),
                None,
            );

            const NUMBER_OF_ASSETS: u64 = 2;
            let node_config = Config {
                database_type: DbType::RocksDb(Some(temp_database_path.clone())),
                ..Config::default()
            };

            let chain_config = ChainConfig {
                chain_name: temp_dir_name.clone(),
                initial_state: None,
                consensus_parameters: Default::default(),
                ..ChainConfig::local_testnet()
            };

            let (coins, _) = setup_multiple_assets_coins(
                wallet.address(),
                NUMBER_OF_ASSETS,
                DEFAULT_NUM_COINS,
                DEFAULT_COIN_AMOUNT,
            );

            let provider =
                setup_test_provider(coins.clone(), vec![], Some(node_config), Some(chain_config))
                    .await?;

            provider.produce_blocks(2, None).await?;

            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .unwrap();

    // The runtime needs to be terminated because the node can currently only be killed when the runtime itself shuts down.

    tokio::runtime::Runtime::new()
        .expect("Tokio runtime failed")
        .block_on(async {
            let node_config = Config {
                database_type: DbType::RocksDb(Some(temp_database_path.clone())),
                ..Config::default()
            };

            let provider = setup_test_provider(vec![], vec![], Some(node_config), None).await?;
            // the same wallet that was used when rocksdb was built. When we connect it to the provider, we expect it to have the same amount of assets
            let mut wallet = WalletUnlocked::new_from_private_key(
                SecretKey::from_str(
                    "0x4433d156e8c53bf5b50af07aa95a29436f29a94e0ccc5d58df8e57bdc8583c32",
                )
                .unwrap(),
                None,
            );

            wallet.set_provider(provider.clone());

            let blocks = provider
                .get_blocks(PaginationRequest {
                    cursor: None,
                    results: 10,
                    direction: PageDirection::Forward,
                })
                .await?
                .results;

            assert_eq!(provider.chain_info().await?.name, temp_dir_name);
            assert_eq!(blocks.len(), 3);
            assert_eq!(
                *wallet.get_balances().await?.iter().next().unwrap().1,
                DEFAULT_COIN_AMOUNT
            );
            assert_eq!(
                *wallet.get_balances().await?.iter().next().unwrap().1,
                DEFAULT_COIN_AMOUNT
            );
            assert_eq!(wallet.get_balances().await?.len(), 2);

            fs::remove_dir_all(
                temp_database_path
                    .parent()
                    .expect("Db parent folder does not exist"),
            )?;

            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .unwrap();
}

#[tokio::test]
async fn can_configure_decoding_of_contract_return() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "packages/fuels/tests/contracts/needs_custom_decoder"
        ),),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet"
        )
    );

    let methods = contract_instance.methods();
    {
        // Single call: Will not work if max_tokens not big enough
        methods.i_return_a_1k_el_array().with_decoder_config(DecoderConfig{max_tokens: 100, ..Default::default()}).call().await.expect_err(
            "Should have failed because there are more tokens than what is supported by default.",
        );
    }
    {
        // Single call: Works when limit is bumped
        let result = methods
            .i_return_a_1k_el_array()
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call()
            .await?
            .value;

        assert_eq!(result, [0; 1000]);
    }
    {
        // Multi call: Will not work if max_tokens not big enough
        MultiContractCallHandler::new(wallet.clone())
        .add_call(methods.i_return_a_1k_el_array())
        .with_decoder_config(DecoderConfig { max_tokens: 100, ..Default::default() })
        .call::<([u8; 1000],)>().await.expect_err(
            "Should have failed because there are more tokens than what is supported by default",
        );
    }
    {
        // Multi call: Works when configured
        MultiContractCallHandler::new(wallet.clone())
            .add_call(methods.i_return_a_1k_el_array())
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call::<([u8; 1000],)>()
            .await
            .unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_contract_submit_and_response() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let contract_methods = contract_instance.methods();

    let submitted_tx = contract_methods.get(1, 2).submit().await?;
    let value = submitted_tx.response().await?.value;

    assert_eq!(value, 3);

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(7);
    let call_handler_2 = contract_methods.get_single(42);

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let handle = multi_call_handler.submit().await?;
    let (val_1, val_2): (u64, u64) = handle.response().await?.value;

    assert_eq!(val_1, 7);
    assert_eq!(val_2, 42);

    Ok(())
}

#[tokio::test]
async fn test_heap_type_multicall() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "TestContract",
                project = "packages/fuels/tests/contracts/contract_test"
            ),
            Contract(
                name = "VectorOutputContract",
                project = "packages/fuels/tests/types/contracts/vector_output"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_instance_2",
            contract = "VectorOutputContract",
            wallet = "wallet"
        ),
    );

    {
        // One heap type at the last position is allowed
        let call_handler_1 = contract_instance.methods().get_single(7);
        let call_handler_2 = contract_instance.methods().get_single(42);
        let call_handler_3 = contract_instance_2.methods().u8_in_vec(3);

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2)
            .add_call(call_handler_3);

        let handle = multi_call_handler.submit().await?;
        let (val_1, val_2, val_3): (u64, u64, Vec<u8>) = handle.response().await?.value;

        assert_eq!(val_1, 7);
        assert_eq!(val_2, 42);
        assert_eq!(val_3, vec![0, 1, 2]);
    }

    {
        let call_handler_1 = contract_instance_2.methods().u8_in_vec(3);
        let call_handler_2 = contract_instance.methods().get_single(7);
        let call_handler_3 = contract_instance_2.methods().u8_in_vec(3);

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2)
            .add_call(call_handler_3);

        let error = multi_call_handler.submit().await.expect_err("Should error");
        assert!(error.to_string().contains(
            "`MultiContractCallHandler` can have only one call that returns a heap type"
        ));
    }

    {
        let call_handler_1 = contract_instance.methods().get_single(7);
        let call_handler_2 = contract_instance_2.methods().u8_in_vec(3);
        let call_handler_3 = contract_instance.methods().get_single(42);

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2)
            .add_call(call_handler_3);

        let error = multi_call_handler.submit().await.expect_err("Should error");
        assert!(error
            .to_string()
            .contains("The contract call with the heap type return must be at the last position"));
    }

    Ok(())
}

#[tokio::test]
async fn heap_types_correctly_offset_in_create_transactions_w_storage_slots() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Predicate(
            name = "MyPredicate",
            project = "packages/fuels/tests/types/predicates/predicate_vector"
        ),),
    );

    let provider = wallet.try_provider()?.clone();
    let data = MyPredicateEncoder::encode_data(18, 24, vec![2, 4, 42]);
    let predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(data)
    .with_provider(provider);

    wallet
        .transfer(
            predicate.address(),
            10_000,
            BASE_ASSET_ID,
            TxPolicies::default(),
        )
        .await?;

    // if the contract is successfully deployed then the predicate was unlocked. This further means
    // the offsets were setup correctly since the predicate uses heap types in its arguments.
    // Storage slots were loaded automatically by default
    Contract::load_from(
        "tests/contracts/storage/out/debug/storage.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxPolicies::default())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_arguments_with_gas_forwarded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "TestContract",
                project = "packages/fuels/tests/contracts/contract_test"
            ),
            Contract(
                name = "VectorOutputContract",
                project = "packages/fuels/tests/types/contracts/vectors"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_instance_2",
            contract = "VectorOutputContract",
            wallet = "wallet"
        ),
    );

    let x = 128;
    let vec_input = vec![0, 1, 2];
    {
        let response = contract_instance
            .methods()
            .get_single(x)
            .call_params(CallParameters::default().with_gas_forwarded(1024))?
            .call()
            .await?;

        assert_eq!(response.value, x);
    }
    {
        contract_instance_2
            .methods()
            .u32_vec(vec_input.clone())
            .call_params(CallParameters::default().with_gas_forwarded(1024))?
            .call()
            .await?;
    }
    {
        let call_handler_1 = contract_instance.methods().get_single(x);
        let call_handler_2 = contract_instance_2.methods().u32_vec(vec_input);

        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let (value, _): (u64, ()) = multi_call_handler.call().await?.value;

        assert_eq!(value, x);
    }

    Ok(())
}
