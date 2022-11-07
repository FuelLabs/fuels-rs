use fuel_gql_client::{
    fuel_tx::{ContractId, Input, Output, TxPointer, UtxoId},
    fuel_vm::{
        consts::{REG_ONE, WORD_SIZE},
        prelude::{GTFArgs, Opcode},
    },
};
use fuels::prelude::*;
use fuels_contract::script::Script;
use fuels_core::tx::Bytes32;

#[tokio::test]
async fn test_transaction_script_workflow() -> Result<(), Error> {
    setup_contract_test!(
        contract_instance,
        wallet,
        "packages/fuels/tests/contracts/contract_test"
    );

    let call_handler = contract_instance.methods().initialize_counter(42);

    let script = call_handler.get_call_execution_script().await?;

    let provider = wallet.get_provider()?;
    let receipts = script.call(provider).await?;

    let response = call_handler.get_response(receipts)?;
    assert_eq!(response.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_multi_call_script_workflow() -> Result<(), Error> {
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

    let provider = &wallet.get_provider()?;
    let script = multi_call_handler.get_call_execution_script().await?;
    let receipts = script.call(provider).await.unwrap();
    let (counter, array) = multi_call_handler
        .get_response::<(u64, [u64; 2])>(receipts)?
        .value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);
    Ok(())
}

#[tokio::test]
async fn test_script_interface() -> Result<(), Error> {
    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_coins = wallet
        .get_provider()?
        .get_contract_balances(&contract_id)
        .await?;
    assert!(contract_coins.is_empty());

    let amount = 100;
    let asset_id = Default::default();
    let tx_parameters = TxParameters::default();
    let zeroes = Bytes32::zeroed();
    let plain_contract_id: ContractId = contract_id.clone().into();

    let mut inputs = vec![Input::contract(
        UtxoId::new(zeroes, 0),
        zeroes,
        zeroes,
        TxPointer::default(),
        plain_contract_id,
    )];
    inputs.extend(
        wallet
            .get_asset_inputs_for_amount(asset_id, amount, 0)
            .await?,
    );

    let outputs = vec![
        Output::contract(0, zeroes, zeroes),
        Output::change(wallet.address().into(), 0, asset_id),
    ];

    let script_data: Vec<u8> = [
        plain_contract_id.to_vec(),
        amount.to_be_bytes().to_vec(),
        asset_id.to_vec(),
    ]
    .into_iter()
    .flatten()
    .collect();

    let script_binary = vec![
        Opcode::gtf(0x10, 0x00, GTFArgs::ScriptData),
        Opcode::ADDI(0x11, 0x10, ContractId::LEN as u16),
        Opcode::LW(0x12, 0x11, 0),
        Opcode::ADDI(0x13, 0x11, WORD_SIZE as u16),
        Opcode::TR(0x10, 0x12, 0x13),
        Opcode::RET(REG_ONE),
    ]
    .into_iter()
    .collect();

    let script = Script::from_binary(
        script_binary,
        tx_parameters,
        Some(script_data),
        Some(inputs),
        Some(outputs),
    );
    let provider = wallet.get_provider()?;
    let _result = script.call(provider).await;

    let contract_balances = wallet
        .get_provider()?
        .get_contract_balances(&contract_id)
        .await?;
    assert_eq!(contract_balances.len(), 1);

    let asset_id_key = format!("{:#x}", BASE_ASSET_ID);
    let balance = contract_balances.get(&asset_id_key).unwrap();
    assert_eq!(*balance, 100);

    Ok(())
}

#[tokio::test]
async fn main_function_arguments() -> Result<(), Error> {
    // ANCHOR: script_with_arguments

    // The abigen is used for the same purpose as with contracts (Rust bindings)
    script_abigen!(
        MyScript,
        "packages/fuels/tests/scripts/script_with_arguments/out/debug/script_with_arguments-abi.json"
    );
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path =
        "../fuels/tests/scripts/script_with_arguments/out/debug/script_with_arguments.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = Bimbam { val: 90 };
    let bam = SugarySnack {
        twix: 100,
        mars: 1000,
    };
    let result = instance.main(bim, bam).await?;
    let expected = Bimbam { val: 2190 };
    assert_eq!(result, expected);
    // ANCHOR_END: script_with_arguments
    Ok(())
}
#[tokio::test]
async fn main_function_generic_arguments() -> Result<(), Error> {
    script_abigen!(
        MyScript,
        "packages/fuels/tests/scripts/script_generic_types/out/debug/script_generic_types-abi.json"
    );
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path = "../fuels/tests/scripts/script_generic_types/out/debug/script_generic_types.bin";
    let instance = MyScript::new(wallet, bin_path);

    let bim = GenericBimbam { val: 90 };
    let bam_comp = GenericBimbam { val: 4342 };
    let bam = GenericSnack {
        twix: bam_comp,
        mars: 1000,
    };
    let result = instance.main(bim.clone(), bam.clone()).await?;
    let expected = (
        GenericSnack {
            twix: GenericBimbam {
                val: bam.mars as u64,
            },
            mars: 2 * bim.val as u32,
        },
        GenericBimbam { val: 255_u8 },
    );
    assert_eq!(result, expected);
    Ok(())
}

#[tokio::test]
async fn main_function_option_result() -> Result<(), Error> {
    script_abigen!(
        MyScript,
        "packages/fuels/tests/scripts/script_option_result_types/out/debug\
        /script_option_result_types-abi.json"
    );
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path =
        "../fuels/tests/scripts/script_option_result_types/out/debug/script_option_result_types.bin";
    let instance = MyScript::new(wallet, bin_path);

    let result = instance.main(Some(42), None).await?;
    assert_eq!(result, Ok(Some(true)));
    let result = instance.main(Some(987), None).await?;
    assert_eq!(result, Ok(None));
    let expected_error = Err(TestError::ZimZam("error".try_into().unwrap()));
    let result = instance.main(None, Some(987)).await?;
    assert_eq!(result, expected_error);
    Ok(())
}

#[tokio::test]
async fn test_script_from_binary_filepath() -> Result<(), Error> {
    // ANCHOR: script_from_binary_filepath
    let path_to_bin = "../fuels/tests/scripts/basic_script/out/debug/basic_script.bin";
    let (provider, _) = setup_test_provider(vec![], vec![], None, None).await;
    // Provide `None` to all other arguments so the function uses the default for each
    let script = Script::from_binary_filepath(path_to_bin, None, None, None, None)?;
    let return_val = script.call(&provider).await?;

    let expected = 29879;
    assert_eq!(return_val[0].val().unwrap(), expected);
    // ANCHOR_END: script_from_binary_filepath
    Ok(())
}

#[tokio::test]
async fn main_function_tuple_types() -> Result<(), Error> {
    script_abigen!(
        MyScript,
        "packages/fuels/tests/scripts/script_tuple_types/out/debug/script_tuple_types-abi.json"
    );
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
    assert_eq!(result, expected);
    Ok(())
}

#[tokio::test]
async fn main_function_vector_arguments() -> Result<(), Error> {
    script_abigen!(
        MyScript,
        "packages/fuels/tests/scripts/script_vector_arguments/out/debug/script_vector_arguments-abi.json"
    );
    let wallet = launch_provider_and_get_wallet().await;
    let bin_path =
        "../fuels/tests/scripts/script_vector_arguments/out/debug/script_vector_arguments.bin";
    let instance = MyScript::new(wallet, bin_path);
    let result = instance.main(vec![1, 2, 3, 4, 5]).await?;
    println!("{:?}", result);
    Ok(())
}
