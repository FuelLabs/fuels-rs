use fuel_asm::{op, Instruction, RegId};
use fuels::{
    core::codec::{DecoderConfig, EncoderConfig},
    prelude::*,
    types::{Identity, Token},
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

fn get_data_offset(binary: &[u8]) -> usize {
    let data_offset: [u8; 8] = binary[8..16].try_into().unwrap();
    u64::from_be_bytes(data_offset) as usize
}

fn our_loader(original_binary: &[u8], blob_id: &BlobId) -> Result<Vec<u8>> {
    let offset = get_data_offset(original_binary);

    let mut data_section = original_binary[offset..].to_vec();
    assert_eq!(data_section[..8], 9000u64.to_be_bytes());

    data_section[..8].copy_from_slice(&10001u64.to_be_bytes());

    let data_section_len = data_section.len();

    const BLOB_ID_SIZE: u16 = 32;
    let get_instructions = |num_of_instructions, num_of_blobs| {
        // There are 2 main steps:
        // 1. Load the blob contents into memory
        // 2. Jump to the beginning of the memory where the blobs were loaded
        // After that the execution continues normally with the loaded contract reading our
        // prepared fn selector and jumps to the selected contract method.
        [
            // 1. Load the blob contents into memory
            // Find the start of the hardcoded blob IDs, which are located after the code ends.
            op::move_(0x10, RegId::PC),
            // 0x10 to hold the address of the current blob ID.
            op::addi(0x10, 0x10, num_of_instructions * Instruction::SIZE as u16),
            // The contract is going to be loaded from the current value of SP onwards, save
            // the location into 0x16 so we can jump into it later on.
            op::move_(0x16, RegId::SP),
            // Loop counter.
            op::movi(0x13, num_of_blobs),
            // LOOP starts here.
            // 0x11 to hold the size of the current blob.
            op::bsiz(0x11, 0x10),
            // Push the blob contents onto the stack.
            op::ldc(0x10, 0, 0x11, 1),
            // Move on to the next blob.
            op::addi(0x10, 0x10, BLOB_ID_SIZE),
            // Decrement the loop counter.
            op::subi(0x13, 0x13, 1),
            // Jump backwards (3+1) instructions if the counter has not reached 0.
            op::jnzb(0x13, RegId::ZERO, 3),
            // TODO: is an immediate here to little, can the data section be bigger?
            op::cfei(data_section_len as u32),
            op::subi(0x11, RegId::SP, data_section_len as u16),
            op::mcpi(0x11, 0x10, data_section_len as u16),
            // 2. Jump into the memory where the contract is loaded.
            // What follows is called _jmp_mem by the sway compiler.
            // Subtract the address contained in IS because jmp will add it back.
            op::sub(0x16, 0x16, RegId::IS),
            // jmp will multiply by 4, so we need to divide to cancel that out.
            op::divi(0x16, 0x16, 4),
            // Jump to the start of the contract we loaded.
            op::jmp(0x16),
        ]
    };

    let num_of_instructions = u16::try_from(get_instructions(0, 0).len())
        .expect("to never have more than u16::MAX instructions");

    let num_of_blobs = 1;

    let instruction_bytes = get_instructions(num_of_instructions, num_of_blobs)
        .into_iter()
        .flat_map(|instruction| instruction.to_bytes());

    let blob_bytes = blob_id.iter().copied();

    Ok(instruction_bytes
        .chain(blob_bytes)
        .chain(data_section)
        .collect())
}

#[tokio::test]
async fn can_be_run_in_blobs() -> Result<()> {
    let binary = std::fs::read("./sway/scripts/script_blobs/out/release/script_blobs.bin").unwrap();

    let wallet = launch_provider_and_get_wallet().await.unwrap();
    let provider = wallet.provider().unwrap().clone();

    let data_section_offset = get_data_offset(&binary);
    let blob = Blob::new(binary[..data_section_offset].to_vec());

    let blob_id = blob.id();

    let mut blob_tb = BlobTransactionBuilder::default().with_blob(blob);

    wallet.adjust_for_fee(&mut blob_tb, 0).await.unwrap();
    blob_tb.add_signer(wallet.clone()).unwrap();

    let tx = blob_tb.build(provider.clone()).await.unwrap();
    provider
        .send_transaction_and_await_commit(tx)
        .await
        .unwrap()
        .check(None)
        .unwrap();

    let new_binary = our_loader(&binary, &blob_id).unwrap();

    let mut tb = ScriptTransactionBuilder::default().with_script(new_binary);

    wallet.adjust_for_fee(&mut tb, 0).await.unwrap();

    tb.add_signer(wallet.clone()).unwrap();

    let tx = tb.build(&provider).await.unwrap();

    let response = provider
        .send_transaction_and_await_commit(tx)
        .await
        .unwrap();

    response.check(None).unwrap();

    Ok(())
}
