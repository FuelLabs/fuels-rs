use fuel_gql_client::{
    fuel_tx::{ContractId, Input, Output, TxPointer, UtxoId},
    fuel_vm::{
        consts::{REG_ONE, WORD_SIZE},
        prelude::{GTFArgs, Opcode},
    },
};
use fuels::prelude::*;
use fuels_contract::script::ScriptBuilder;
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
    assert!(script.tx.is_script());

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

    let script = vec![
        Opcode::gtf(0x10, 0x00, GTFArgs::ScriptData),
        Opcode::ADDI(0x11, 0x10, ContractId::LEN as u16),
        Opcode::LW(0x12, 0x11, 0),
        Opcode::ADDI(0x13, 0x11, WORD_SIZE as u16),
        Opcode::TR(0x10, 0x12, 0x13),
        Opcode::RET(REG_ONE),
    ]
    .into_iter()
    .collect();

    ScriptBuilder::new()
        .set_gas_price(tx_parameters.gas_price)
        .set_gas_limit(tx_parameters.gas_limit)
        .set_maturity(tx_parameters.maturity)
        .set_script(script)
        .set_script_data(script_data)
        .set_inputs(inputs.to_vec())
        .set_outputs(outputs.to_vec())
        .set_amount(amount)
        .build(&wallet)
        .await?
        .call(wallet.get_provider()?)
        .await?;

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
