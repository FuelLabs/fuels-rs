use std::time::Duration;

use fuels::{
    core::{
        codec::{DecoderConfig, EncoderConfig},
        traits::Tokenizable,
    },
    prelude::*,
    programs::executable::Executable,
    types::{Bits256, Identity},
};

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
    let estimated_gas_used = script_instance
        .main(a, b)
        .estimate_transaction_cost(tolerance, block_horizon)
        .await?
        .gas_used;

    let gas_used = script_instance.main(a, b).call().await?.gas_used;

    assert_eq!(estimated_gas_used, gas_used);

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

    let provider = wallet.try_provider()?.clone();
    let mut receiver = WalletUnlocked::new_random(None);
    receiver.set_provider(provider);

    let amount = 1000;
    let asset_id = AssetId::zeroed();
    let script_call = script_instance.main(
        amount,
        asset_id,
        Identity::Address(receiver.address().into()),
    );
    let inputs = wallet
        .get_asset_inputs_for_amount(asset_id, amount, None)
        .await?;
    let _ = script_call
        .with_inputs(inputs)
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
            .main()
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
            .main()
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call()
            .await?
            .value;

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
    let provider = wallet.try_provider()?;

    // ANCHOR: script_call_tb
    let script_call_handler = script_instance.main(1, 2);

    let mut tb = script_call_handler.transaction_builder().await?;

    // customize the builder...

    wallet.adjust_for_fee(&mut tb, 0).await?;
    tb.add_signer(wallet.clone())?;

    let tx = tb.build(provider).await?;

    let tx_id = provider.send_transaction(tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let tx_status = provider.tx_status(&tx_id).await?;

    let response = script_call_handler.get_response_from(tx_status)?;

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
            .simulate(Execution::Realistic)
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
    let provider = wallet.provider().cloned();

    let no_funds_wallet = WalletUnlocked::new_random(provider);
    let script_instance = script_instance.with_account(no_funds_wallet);

    let value = script_instance
        .main(1000, 2000)
        .simulate(Execution::StateReadOnly)
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
    let provider = wallet.try_provider()?.clone();

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
        .deploy(&wallet, TxPolicies::default())
        .await?;

    let contract_binary = "sway/contracts/proxy/out/release/proxy.bin";

    let proxy_id = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .convert_to_loader(100)?
        .deploy(&wallet, TxPolicies::default())
        .await?;

    let proxy = MyProxy::new(proxy_id.clone(), wallet.clone());
    proxy
        .methods()
        .set_target_contract(contract_id.clone())
        .call()
        .await?;

    let mut my_script = my_script;
    let result = my_script
        .convert_into_loader()
        .await?
        .main(proxy_id.clone())
        .with_contract_ids(&[contract_id, proxy_id])
        .call()
        .await?;

    assert!(result.value);

    Ok(())
}
