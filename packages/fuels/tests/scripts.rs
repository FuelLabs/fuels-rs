use fuels::{prelude::*, types::Bits256};

#[tokio::test]
async fn test_transaction_script_workflow() -> Result<()> {
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

    let call_handler = contract_instance.methods().initialize_counter(42);

    let response = call_handler.call().await?;
    assert!(response.tx_id.is_some());
    assert_eq!(response.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_multi_call_script_workflow() -> Result<()> {
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

    let response = multi_call_handler.call::<(u64, [u64; 2])>().await?;
    assert!(response.tx_id.is_some());
    let (counter, array) = response.value;
    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);
    Ok(())
}

#[tokio::test]
async fn main_function_arguments() -> Result<()> {
    // ANCHOR: script_with_arguments
    // The abigen is used for the same purpose as with contracts (Rust bindings)
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/scripts/arguments/out/debug/arguments-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/arguments/out/debug/arguments.bin";
    let script_instance = MyScript::new(wallet, bin_path);

    let bim = Bimbam { val: 90 };
    let bam = SugarySnack {
        twix: 100,
        mars: 1000,
    };

    let result = script_instance.main(bim, bam).call().await?;

    let expected = Bimbam { val: 2190 };
    assert_eq!(result.value, expected);
    // ANCHOR_END: script_with_arguments
    Ok(())
}

#[tokio::test]
async fn script_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let tolerance = 0.0;

    let a = 4u64;
    let b = 2u32;
    let estimated_gas_used = script_instance
        .main(a, b)
        .estimate_transaction_cost(Some(tolerance))
        .await?
        .gas_used;

    let gas_used = script_instance.main(a, b).call().await?.gas_used;

    assert_eq!(estimated_gas_used, gas_used);
    Ok(())
}

#[tokio::test]
async fn test_basic_script_with_tx_parameters() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "bimbam_script",
            project = "packages/fuels/tests/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "bimbam_script",
            wallet = "wallet"
        )
    );

    let a = 1000u64;
    let b = 2000u32;
    let result = script_instance.main(a, b).call().await?;
    assert_eq!(result.value, "hello");
    // ANCHOR: script_with_tx_params
    let parameters = TxParameters::default()
        .with_gas_price(1)
        .with_gas_limit(1_000_000);
    let result = script_instance
        .main(a, b)
        .tx_params(parameters)
        .call()
        .await?;
    // ANCHOR_END: script_with_tx_params
    assert_eq!(result.value, "hello");

    Ok(())
}

#[tokio::test]
async fn test_script_call_with_non_default_max_input() -> Result<()> {
    use fuels::{tx::ConsensusParameters, types::coin::Coin};

    let consensus_parameters_config = ConsensusParameters::DEFAULT.with_max_inputs(128);
    let chain_config = ChainConfig {
        transaction_parameters: consensus_parameters_config,
        ..ChainConfig::default()
    };

    let mut wallet = WalletUnlocked::new_random(None);

    let coins: Vec<Coin> = setup_single_asset_coins(
        wallet.address(),
        Default::default(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );

    let (fuel_client, _, consensus_parameters) =
        setup_test_client(coins, vec![], None, Some(chain_config)).await;
    let provider = Provider::new(fuel_client, consensus_parameters);
    assert_eq!(consensus_parameters, consensus_parameters_config);
    wallet.set_provider(provider.clone());

    setup_program_test!(
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let a = 4u64;
    let b = 2u32;

    let result = script_instance.main(a, b).call().await?;

    assert_eq!(result.value, "heyoo");
    Ok(())
}

#[tokio::test]
async fn test_script_signing() -> Result<()> {
    let wallet_config = WalletsConfig::new(Some(1), None, None);
    let provider_config = Config {
        utxo_validation: true,
        ..Config::local_node()
    };

    let wallets =
        launch_custom_provider_and_get_wallets(wallet_config, Some(provider_config), None).await;
    let wallet = wallets.first().unwrap();

    setup_program_test!(
        Abigen(Script(
            name = "BimBamScript",
            project = "packages/fuels/tests/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "BimBamScript",
            wallet = "wallet"
        )
    );

    let a = 1000u64;
    let b = 2000u32;

    let result = script_instance.main(a, b).call().await?;

    assert_eq!(result.value, "hello");

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "transfer_script",
            project = "packages/fuels/tests/scripts/transfer_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "transfer_script",
            wallet = "wallet"
        )
    );

    let provider = wallet.try_provider()?.clone();
    let mut receiver = WalletUnlocked::new_random(None);
    receiver.set_provider(provider);

    let amount = 1000;
    let asset_id = BASE_ASSET_ID;
    let script_call = script_instance.main(amount, asset_id.into(), receiver.address());
    let inputs = wallet
        .get_asset_inputs_for_amount(BASE_ASSET_ID, amount, None)
        .await?;
    let _ = script_call
        .with_inputs(inputs)
        .estimate_tx_dependencies(None)
        .await?
        .call()
        .await?;

    let receiver_balance = receiver.get_asset_balance(&BASE_ASSET_ID).await?;
    assert_eq!(receiver_balance, amount);

    Ok(())
}

#[tokio::test]
async fn test_script_struct() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_struct"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let my_struct = MyStruct {
        number: 42,
        boolean: true,
    };
    let response = script_instance.main(my_struct).call().await?;

    assert_eq!(response.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_script_enum() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_enum"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let my_enum = MyEnum::Two;
    let response = script_instance.main(my_enum).call().await?;

    assert_eq!(response.value, 2);
    Ok(())
}

#[tokio::test]
async fn test_script_array() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_array"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let my_array: [u64; 4] = [1, 2, 3, 4];
    let response = script_instance.main(my_array).call().await?;

    assert_eq!(response.value, 10);
    Ok(())
}

#[tokio::test]
async fn test_script_b256() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_b256"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let my_b256 = Bits256([1; 32]);
    let response = script_instance.main(my_b256).call().await?;

    assert!(response.value);
    Ok(())
}

#[tokio::test]
async fn test_script_submit_and_response() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "packages/fuels/tests/scripts/script_struct"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let my_struct = MyStruct {
        number: 42,
        boolean: true,
    };

    let handle = script_instance.main(my_struct).submit().await?;
    let response = handle.response().await?;

    assert_eq!(response.value, 42);
    Ok(())
}
