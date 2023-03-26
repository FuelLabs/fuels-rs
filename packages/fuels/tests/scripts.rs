use fuels::prelude::*;

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

    let tx = call_handler.build_tx().await?;
    let provider = wallet.try_provider()?;
    let receipts = provider.send_transaction(&tx).await?;

    let response = call_handler.get_response(receipts)?;
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

    let provider = wallet.try_provider()?;
    let tx = multi_call_handler.build_tx().await?;
    let receipts = provider.send_transaction(&tx).await?;
    let (counter, array) = multi_call_handler
        .get_response::<(u64, [u64; 2])>(receipts)?
        .value;

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
        .set_gas_price(1)
        .set_gas_limit(10_000);
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
    use fuels::tx::ConsensusParameters;
    use fuels_types::coin::Coin;

    let consensus_parameters_config = ConsensusParameters::DEFAULT.with_max_inputs(128);

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
