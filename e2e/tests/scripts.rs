use std::time::Duration;

use fuel_tx::Output;
use fuels::{
    accounts::signers::private_key::PrivateKeySigner,
    client::{PageDirection, PaginationRequest},
    core::{
        Configurables,
        codec::{DecoderConfig, EncoderConfig},
        traits::Tokenizable,
    },
    prelude::*,
    programs::{DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE, executable::Executable},
    types::{Bits256, Identity},
};
use rand::thread_rng;

#[tokio::test]
async fn main_function_arguments() -> Result<()> {
    // ANCHOR: script_with_arguments
    // The abigen is used for the same purpose as with contracts (Rust bindings)
    abigen!(Script(
        name = "MyScript",
        abi = "e2e/sway/scripts/arguments/out/release/arguments-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await?;
    let bin_path = "sway/scripts/arguments/out/release/arguments.bin";
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
            project = "e2e/sway/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    let tolerance = Some(0.0);
    let block_horizon = Some(1);

    let a = 4u64;
    let b = 2u32;
    let estimated_total_gas = script_instance
        .main(a, b)
        .estimate_transaction_cost(tolerance, block_horizon)
        .await?
        .total_gas;

    let total_gas = script_instance.main(a, b).call().await?.tx_status.total_gas;

    assert_eq!(estimated_total_gas, total_gas);

    Ok(())
}

#[tokio::test]
async fn test_basic_script_with_tx_policies() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "bimbam_script",
            project = "e2e/sway/scripts/basic_script"
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

    // ANCHOR: script_with_tx_policies
    let tx_policies = TxPolicies::default().with_script_gas_limit(1_000_000);
    let result = script_instance
        .main(a, b)
        .with_tx_policies(tx_policies)
        .call()
        .await?;
    // ANCHOR_END: script_with_tx_policies
    assert_eq!(result.value, "hello");

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "transfer_script",
            project = "e2e/sway/scripts/transfer_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "transfer_script",
            wallet = "wallet"
        )
    );

    let provider = wallet.provider().clone();
    let receiver = Wallet::random(&mut thread_rng(), provider);

    let amount = 1000;
    let asset_id = AssetId::zeroed();
    let script_call = script_instance.main(amount, asset_id, Identity::Address(receiver.address()));
    let inputs = wallet
        .get_asset_inputs_for_amount(asset_id, amount.into(), None)
        .await?;
    let output = Output::change(wallet.address(), 0, asset_id);
    let _ = script_call
        .with_inputs(inputs)
        .with_outputs(vec![output])
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call()
        .await?;

    let receiver_balance = receiver.get_asset_balance(&asset_id).await?;
    assert_eq!(receiver_balance, amount);

    Ok(())
}

#[tokio::test]
async fn test_script_struct() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_struct"
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
            project = "e2e/sway/scripts/script_enum"
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
            project = "e2e/sway/scripts/script_array"
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
async fn can_configure_decoder_on_script_call() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_needs_custom_decoder"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );

    {
        // Will fail if max_tokens too low
        script_instance
            .main(false)
            .with_decoder_config(DecoderConfig {
                max_tokens: 101,
                ..Default::default()
            })
            .call()
            .await
            .expect_err(
                "Should fail because return type has more tokens than what is allowed by default",
            );
    }
    {
        // When the token limit is bumped should pass
        let response = script_instance
            .main(false)
            .with_decoder_config(DecoderConfig {
                max_tokens: 1002,
                ..Default::default()
            })
            .call()
            .await?
            .value
            .unwrap();

        assert_eq!(response, [0u8; 1000]);
    }

    Ok(())
}

#[tokio::test]
async fn test_script_submit_and_response() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/script_struct"
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

    // ANCHOR: submit_response_script
    let submitted_tx = script_instance.main(my_struct).submit().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let value = submitted_tx.response().await?.value;
    // ANCHOR_END: submit_response_script

    assert_eq!(value, 42);
    Ok(())
}

#[tokio::test]
async fn test_script_transaction_builder() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );
    let provider = wallet.provider();

    // ANCHOR: script_call_tb
    let script_call_handler = script_instance.main(1, 2);

    let mut tb = script_call_handler.transaction_builder().await?;

    // customize the builder...

    wallet.adjust_for_fee(&mut tb, 0).await?;
    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(provider).await?;

    let tx_id = provider.send_transaction(tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let tx_status = provider.tx_status(&tx_id).await?;

    let response = script_call_handler.get_response(tx_status)?;

    assert_eq!(response.value, "hello");
    // ANCHOR_END: script_call_tb

    Ok(())
}

#[tokio::test]
async fn script_encoder_config_is_applied() {
    abigen!(Script(
        name = "MyScript",
        abi = "e2e/sway/scripts/basic_script/out/release/basic_script-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await.expect("");
    let bin_path = "sway/scripts/basic_script/out/release/basic_script.bin";

    let script_instance_without_encoder_config = MyScript::new(wallet.clone(), bin_path);
    {
        let _encoding_ok = script_instance_without_encoder_config
            .main(1, 2)
            .call()
            .await
            .expect("should not fail as it uses the default encoder config");
    }
    {
        let encoder_config = EncoderConfig {
            max_tokens: 1,
            ..Default::default()
        };
        let script_instance_with_encoder_config =
            MyScript::new(wallet.clone(), bin_path).with_encoder_config(encoder_config);

        // uses 2 tokens when 1 is the limit
        let encoding_error = script_instance_with_encoder_config
            .main(1, 2)
            .call()
            .await
            .expect_err("should error");

        assert!(encoding_error.to_string().contains(
            "cannot encode script call arguments: codec: token limit `1` reached while encoding"
        ));

        let encoding_error = script_instance_with_encoder_config
            .main(1, 2)
            .simulate(Execution::realistic())
            .await
            .expect_err("should error");

        assert!(encoding_error.to_string().contains(
            "cannot encode script call arguments: codec: token limit `1` reached while encoding"
        ));
    }
}
#[tokio::test]
async fn simulations_can_be_made_without_coins() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Script(
            name = "MyScript",
            project = "e2e/sway/scripts/basic_script"
        )),
        LoadScript(
            name = "script_instance",
            script = "MyScript",
            wallet = "wallet"
        )
    );
    let provider = wallet.provider().clone();

    let no_funds_wallet = Wallet::random(&mut thread_rng(), provider);
    let script_instance = script_instance.with_account(no_funds_wallet);

    let value = script_instance
        .main(1000, 2000)
        .simulate(Execution::state_read_only())
        .await?
        .value;

    assert_eq!(value.as_ref(), "hello");

    Ok(())
}

#[tokio::test]
async fn can_be_run_in_blobs_builder() -> Result<()> {
    abigen!(Script(
        abi = "e2e/sway/scripts/script_blobs/out/release/script_blobs-abi.json",
        name = "MyScript"
    ));

    let binary_path = "./sway/scripts/script_blobs/out/release/script_blobs.bin";
    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.provider().clone();

    // ANCHOR: preload_low_level
    let regular = Executable::load_from(binary_path)?;

    let configurables = MyScriptConfigurables::default().with_SECRET_NUMBER(10001)?;
    let loader = regular
        .convert_to_loader()?
        .with_configurables(configurables);

    // The Blob must be uploaded manually, otherwise the script code will revert.
    loader.upload_blob(wallet.clone()).await?;

    let encoder = fuels::core::codec::ABIEncoder::default();
    let token = MyStruct {
        field_a: MyEnum::B(99),
        field_b: Bits256([17; 32]),
    }
    .into_token();
    let data = encoder.encode(&[token])?;

    let mut tb = ScriptTransactionBuilder::default()
        .with_script(loader.code())
        .with_script_data(data);

    wallet.adjust_for_fee(&mut tb, 0).await?;

    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(&provider).await?;

    let response = provider.send_transaction_and_await_commit(tx).await?;

    response.check(None)?;
    // ANCHOR_END: preload_low_level

    Ok(())
}

#[tokio::test]
async fn can_be_run_in_blobs_high_level() -> Result<()> {
    setup_program_test!(
        Abigen(Script(
            project = "e2e/sway/scripts/script_blobs",
            name = "MyScript"
        )),
        Wallets("wallet"),
        LoadScript(name = "my_script", script = "MyScript", wallet = "wallet")
    );

    let configurables = MyScriptConfigurables::default().with_SECRET_NUMBER(10001)?;
    let mut my_script = my_script.with_configurables(configurables);

    let arg = MyStruct {
        field_a: MyEnum::B(99),
        field_b: Bits256([17; 32]),
    };
    let secret = my_script
        .convert_into_loader()
        .await?
        .main(arg)
        .call()
        .await?
        .value;

    assert_eq!(secret, 10001);

    Ok(())
}

#[tokio::test]
async fn high_level_blob_upload_sets_max_fee_tolerance() -> Result<()> {
    let node_config = NodeConfig {
        starting_gas_price: 1000000000,
        ..Default::default()
    };
    let signer = PrivateKeySigner::random(&mut thread_rng());
    let coins = setup_single_asset_coins(signer.address(), AssetId::zeroed(), 1, u64::MAX);
    let provider = setup_test_provider(coins, vec![], Some(node_config), None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    setup_program_test!(
        Abigen(Script(
            project = "e2e/sway/scripts/script_blobs",
            name = "MyScript"
        )),
        LoadScript(name = "my_script", script = "MyScript", wallet = "wallet")
    );

    let loader = Executable::from_bytes(std::fs::read(
        "sway/scripts/script_blobs/out/release/script_blobs.bin",
    )?)
    .convert_to_loader()?;

    let zero_tolerance_fee = {
        let mut tb = BlobTransactionBuilder::default()
            .with_blob(loader.blob())
            .with_max_fee_estimation_tolerance(0.);

        wallet.adjust_for_fee(&mut tb, 0).await?;

        wallet.add_witnesses(&mut tb)?;
        let tx = tb.build(&provider).await?;
        tx.max_fee().unwrap()
    };

    let mut my_script = my_script;
    my_script.convert_into_loader().await?;

    let max_fee_of_sent_blob_tx = provider
        .get_transactions(PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        })
        .await?
        .results
        .into_iter()
        .find_map(|tx| {
            if let TransactionType::Blob(blob_transaction) = tx.transaction {
                blob_transaction.max_fee()
            } else {
                None
            }
        })
        .unwrap();

    assert_eq!(
        max_fee_of_sent_blob_tx,
        (zero_tolerance_fee as f32 * (1.0 + DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE)).ceil() as u64,
        "the blob upload tx should have had the max fee increased by the default estimation tolerance"
    );

    Ok(())
}

#[tokio::test]
async fn no_data_section_blob_run() -> Result<()> {
    setup_program_test!(
        Abigen(Script(
            project = "e2e/sway/scripts/empty",
            name = "MyScript"
        )),
        Wallets("wallet"),
        LoadScript(name = "my_script", script = "MyScript", wallet = "wallet")
    );

    let mut my_script = my_script;

    // ANCHOR: preload_high_level
    my_script.convert_into_loader().await?.main().call().await?;
    // ANCHOR_END: preload_high_level

    Ok(())
}

#[tokio::test]
async fn loader_script_calling_loader_proxy() -> Result<()> {
    setup_program_test!(
        Abigen(
            Contract(
                name = "MyContract",
                project = "e2e/sway/contracts/huge_contract"
            ),
            Contract(name = "MyProxy", project = "e2e/sway/contracts/proxy"),
            Script(name = "MyScript", project = "e2e/sway/scripts/script_proxy"),
        ),
        Wallets("wallet"),
        LoadScript(name = "my_script", script = "MyScript", wallet = "wallet")
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";

    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?;

    let contract_id = contract
        .convert_to_loader(100)?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let contract_binary = "sway/contracts/proxy/out/release/proxy.bin";

    let proxy_id = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .convert_to_loader(100)?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let proxy = MyProxy::new(proxy_id, wallet.clone());
    proxy
        .methods()
        .set_target_contract(contract_id)
        .call()
        .await?;

    let mut my_script = my_script;
    let result = my_script
        .convert_into_loader()
        .await?
        .main(proxy_id)
        .with_contract_ids(&[contract_id, proxy_id])
        .call()
        .await?;

    assert!(result.value);

    Ok(())
}

#[tokio::test]
async fn loader_can_be_presented_as_a_normal_script_with_shifted_configurables() -> Result<()> {
    abigen!(Script(
        abi = "e2e/sway/scripts/script_blobs/out/release/script_blobs-abi.json",
        name = "MyScript"
    ));

    let binary_path = "./sway/scripts/script_blobs/out/release/script_blobs.bin";
    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.provider().clone();

    let regular = Executable::load_from(binary_path)?;

    let configurables = MyScriptConfigurables::default().with_SECRET_NUMBER(10001)?;
    let loader = regular.clone().convert_to_loader()?;

    // The Blob must be uploaded manually, otherwise the script code will revert.
    loader.upload_blob(wallet.clone()).await?;

    let encoder = fuels::core::codec::ABIEncoder::default();
    let token = MyStruct {
        field_a: MyEnum::B(99),
        field_b: Bits256([17; 32]),
    }
    .into_token();
    let data = encoder.encode(&[token])?;

    let configurables: Configurables = configurables.into();

    let offset = regular
        .configurables_offset_in_code()?
        .unwrap_or_else(|| regular.data_offset_in_code().unwrap());

    let shifted_configurables = configurables
        .with_shifted_offsets(-(offset as i64))
        .unwrap()
        .with_shifted_offsets(loader.configurables_offset_in_code() as i64)
        .unwrap();

    let loader_posing_as_normal_script =
        Executable::from_bytes(loader.code()).with_configurables(shifted_configurables);

    let mut tb = ScriptTransactionBuilder::default()
        .with_script(loader_posing_as_normal_script.code())
        .with_script_data(data);

    wallet.adjust_for_fee(&mut tb, 0).await?;

    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(&provider).await?;

    let response = provider.send_transaction_and_await_commit(tx).await?;

    response.check(None)?;

    Ok(())
}

#[tokio::test]
async fn script_call_respects_maturity_and_expiration() -> Result<()> {
    abigen!(Script(
        name = "MyScript",
        abi = "e2e/sway/scripts/basic_script/out/release/basic_script-abi.json"
    ));
    let wallet = launch_provider_and_get_wallet().await.expect("");
    let provider = wallet.provider().clone();
    let bin_path = "sway/scripts/basic_script/out/release/basic_script.bin";

    let script_instance = MyScript::new(wallet, bin_path);

    let maturity = 10;
    let expiration = 20;
    let call_handler = script_instance.main(1, 2).with_tx_policies(
        TxPolicies::default()
            .with_maturity(maturity)
            .with_expiration(expiration),
    );

    {
        let err = call_handler
            .clone()
            .call()
            .await
            .expect_err("maturity not reached");

        assert!(err.to_string().contains("TransactionMaturity"));
    }
    {
        provider.produce_blocks(15, None).await?;
        call_handler
            .clone()
            .call()
            .await
            .expect("should succeed. Block height between `maturity` and `expiration`");
    }
    {
        provider.produce_blocks(15, None).await?;
        let err = call_handler.call().await.expect_err("expiration reached");

        assert!(err.to_string().contains("TransactionExpiration"));
    }

    Ok(())
}

#[tokio::test]
async fn script_tx_input_output() -> Result<()> {
    let [wallet_1, wallet_2] = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(2), Some(10), Some(1000)),
        None,
        None,
    )
    .await?
    .try_into()
    .unwrap();

    abigen!(Script(
        name = "TxScript",
        abi = "e2e/sway/scripts/script_tx_input_output/out/release/script_tx_input_output-abi.json"
    ));
    let script_binary =
        "sway/scripts/script_tx_input_output/out/release/script_tx_input_output.bin";

    // Set `wallet_1` as the custom input owner
    let configurables = TxScriptConfigurables::default().with_OWNER(wallet_1.address())?;

    let script_instance =
        TxScript::new(wallet_2.clone(), script_binary).with_configurables(configurables);

    let asset_id = AssetId::zeroed();

    {
        let custom_inputs = wallet_1
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .into_iter()
            .take(1)
            .collect();

        let custom_output = vec![Output::change(wallet_1.address(), 0, asset_id)];

        // Input at first position is a coin owned by wallet_1
        // Output at first position is change to wallet_1
        // ANCHOR: script_custom_inputs_outputs
        let _ = script_instance
            .main(0, 0)
            .with_inputs(custom_inputs)
            .with_outputs(custom_output)
            .add_signer(wallet_1.signer().clone())
            .call()
            .await?;
        // ANCHOR_END: script_custom_inputs_outputs
    }
    {
        // Input at first position is not a coin owned by wallet_1
        let err = script_instance.main(0, 0).call().await.unwrap_err();

        assert!(err.to_string().contains("wrong owner"));

        let custom_input = wallet_1
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .pop()
            .unwrap();

        // Input at first position is a coin owned by wallet_1
        // Output at first position is not change to wallet_1
        let err = script_instance
            .main(0, 0)
            .with_inputs(vec![custom_input])
            .add_signer(wallet_1.signer().clone())
            .call()
            .await
            .unwrap_err();

        assert!(err.to_string().contains("wrong change address"));
    }

    Ok(())
}
