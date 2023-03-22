use fuels::prelude::*;

#[tokio::test]
async fn test_transaction_script_workflow() -> Result<()> {
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
    abigen!(Script(name="MyScript", abi="packages/fuels/tests/scripts/script_with_arguments/out/debug/script_with_arguments-abi.json"));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path =
        "../fuels/tests/scripts/script_with_arguments/out/debug/script_with_arguments.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = Bimbam { val: 90 };
    let bam = SugarySnack {
        twix: 100,
        mars: 1000,
    };
    let result = instance.main(bim, bam).call().await?;
    let expected = Bimbam { val: 2190 };
    assert_eq!(result.value, expected);
    // ANCHOR_END: script_with_arguments
    Ok(())
}

#[tokio::test]
async fn main_function_generic_arguments() -> Result<()> {
    abigen!(Script(name="MyScript", abi="packages/fuels/tests/scripts/script_generic_types/out/debug/script_generic_types-abi.json"));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_generic_types/out/debug/script_generic_types.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = GenericBimbam { val: 90 };
    let bam_comp = GenericBimbam { val: 4342 };
    let bam = GenericSnack {
        twix: bam_comp,
        mars: 1000,
    };
    let result = instance.main(bim.clone(), bam.clone()).call().await?;
    let expected = (
        GenericSnack {
            twix: GenericBimbam {
                val: bam.mars as u64,
            },
            mars: 2 * bim.val as u32,
        },
        GenericBimbam { val: 255_u8 },
    );
    assert_eq!(result.value, expected);
    Ok(())
}

#[tokio::test]
async fn main_function_option_result() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/scripts/script_option_result_types/out/debug\
        /script_option_result_types-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path =
        "../fuels/tests/scripts/script_option_result_types/out/debug/script_option_result_types.bin";
    let instance = MyScript::new(wallet, bin_path);

    let result = instance.main(Some(42), None).call().await?;
    assert_eq!(result.value, Ok(Some(true)));
    let result = instance.main(Some(987), None).call().await?;
    assert_eq!(result.value, Ok(None));
    let expected_error = Err(TestError::ZimZam("error".try_into().unwrap()));
    let result = instance.main(None, Some(987)).call().await?;
    assert_eq!(result.value, expected_error);
    Ok(())
}

#[tokio::test]
async fn main_function_tuple_types() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi =
            "packages/fuels/tests/scripts/script_tuple_types/out/debug/script_tuple_types-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_tuple_types/out/debug/script_tuple_types.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = Bim { bim: 90 };
    let bam = Bam {
        bam: "itest".try_into()?,
    };
    let boum = Boum { boum: true };
    let result = instance
        .main(
            (bim, bam, boum),
            Bam {
                bam: "secod".try_into()?,
            },
        )
        .call()
        .await?;
    let expected = (
        (
            Boum { boum: true },
            Bim { bim: 193817 },
            Bam {
                bam: "hello".try_into()?,
            },
        ),
        42242,
    );
    assert_eq!(result.value, expected);

    Ok(())
}

#[tokio::test]
async fn main_function_vector_arguments() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/scripts/script_vectors/out/debug/script_vectors-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_vectors/out/debug/script_vectors.bin";
    let instance = MyScript::new(wallet, bin_path);

    let u32_vec = vec![0, 1, 2];
    let vec_in_vec = vec![vec![0, 1, 2], vec![0, 1, 2]];
    let struct_in_vec = vec![SomeStruct { a: 0 }, SomeStruct { a: 1 }];
    let vec_in_struct = SomeStruct { a: vec![0, 1, 2] };
    let array_in_vec = vec![[0u64, 1u64], [0u64, 1u64]];
    let vec_in_array = [vec![0, 1, 2], vec![0, 1, 2]];
    let vec_in_enum = SomeEnum::a(vec![0, 1, 2]);
    let enum_in_vec = vec![SomeEnum::a(0), SomeEnum::a(1)];

    let tuple_in_vec = vec![(0, 0), (1, 1)];
    let vec_in_tuple = (vec![0, 1, 2], vec![0, 1, 2]);
    let vec_in_a_vec_in_a_struct_in_a_vec = vec![
        SomeStruct {
            a: vec![vec![0, 1, 2], vec![3, 4, 5]],
        },
        SomeStruct {
            a: vec![vec![6, 7, 8], vec![9, 10, 11]],
        },
    ];

    let result = instance
        .main(
            u32_vec,
            vec_in_vec,
            struct_in_vec,
            vec_in_struct,
            array_in_vec,
            vec_in_array,
            vec_in_enum,
            enum_in_vec,
            tuple_in_vec,
            vec_in_tuple,
            vec_in_a_vec_in_a_struct_in_a_vec,
        )
        .call()
        .await?;

    assert!(result.value);

    Ok(())
}

#[tokio::test]
async fn test_basic_script_with_tx_parameters() -> Result<()> {
    abigen!(Script(
        name = "bimbam_script",
        abi = "packages/fuels/tests/scripts/basic_script/out/debug/basic_script-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/basic_script/out/debug/basic_script.bin";
    let instance = bimbam_script::new(wallet.clone(), bin_path);

    let a = 1000u64;
    let b = 2000u32;
    let result = instance.main(a, b).call().await?;
    assert_eq!(result.value, "hello");
    // ANCHOR: script_with_tx_params
    let parameters = TxParameters::default()
        .set_gas_price(1)
        .set_gas_limit(10_000);
    let result = instance.main(a, b).tx_params(parameters).call().await?;
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

    abigen!(Script(
        name = "MyScript",
        abi = "packages/fuels/tests/scripts/script_vector/out/debug/script_vector-abi.json"
    ));

    let bin_path = "../fuels/tests/scripts/script_vector/out/debug/script_vector.bin";
    let instance = MyScript::new(wallet, bin_path);

    let a = 2u32;
    let b = 4u64;
    let u64_vec: Vec<u64> = vec![1024, 2048, 4096];

    let result = instance.main(a, b, u64_vec.clone()).call().await?;

    assert_eq!(result.value, u64_vec[2]);

    Ok(())
}

#[tokio::test]
async fn test_script_raw_slice() -> Result<()> {
    abigen!(Script(
        name = "BimBamScript",
        abi = "packages/fuels/tests/scripts/script_raw_slice/out/debug/script_raw_slice-abi.json",
    ));

    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_raw_slice/out/debug/script_raw_slice.bin";
    let instance = BimBamScript::new(wallet.clone(), bin_path);

    for length in 0..=10 {
        let response = instance.main(length).call().await?;
        assert_eq!(response.value, (0..length).collect::<Vec<_>>());
    }
    Ok(())
}

#[tokio::test]
async fn test_script_signing() -> Result<()> {
    abigen!(Script(
        name = "BimBamScript",
        abi = "packages/fuels/tests/scripts/basic_script/out/debug/basic_script-abi.json"
    ));

    let wallet_config = WalletsConfig::new(Some(1), None, None);
    let provider_config = Config {
        utxo_validation: true,
        ..Config::local_node()
    };

    let wallets =
        launch_custom_provider_and_get_wallets(wallet_config, Some(provider_config), None).await;
    let wallet = wallets.first().unwrap();

    let bin_path = "../fuels/tests/scripts/basic_script/out/debug/basic_script.bin";
    let instance = BimBamScript::new(wallet.clone(), bin_path);

    let a = 1000u64;
    let b = 2000u32;

    let result = instance.main(a, b).call().await?;

    assert_eq!(result.value, "hello");

    Ok(())
}
