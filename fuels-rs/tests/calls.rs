use forc::util::constants;
use fuel_asm::Opcode;
use fuel_tx::{Receipt, Salt, Transaction};
use fuel_types::{Bytes32, ContractId};
use fuel_vm::consts::REG_ZERO;
use fuels_rs::contract::{CompiledContract, Contract};
use fuels_rs::script::Script;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[tokio::test]
async fn script_call() {
    // Compile the script we will be calling
    let compiled =
        Script::compile_sway_script("../fuels-abigen-macro/tests/test_projects/simple_script")
            .unwrap();

    let tx = Transaction::Script {
        gas_price: 0,
        gas_limit: 1_000_000,
        maturity: 0,
        receipts_root: Default::default(),
        script: compiled.raw, // Here we pass the compiled script into the transaction
        script_data: vec![],
        inputs: vec![],
        outputs: vec![],
        witnesses: vec![vec![].into()],
        metadata: None,
    };

    let script = Script::new(tx, constants::DEFAULT_NODE_URL.to_string());

    let result = script.call().await.unwrap();

    let expected_receipt = Receipt::Return {
        id: ContractId::new([0u8; 32]),
        val: 0,
        pc: 840,
        is: 464,
    };

    assert_eq!(expected_receipt, result[0]);
}

#[tokio::test]
async fn contract_call() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // We're going to deploy the following contract:
    let program: Vec<u8> = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0x2a),
        Opcode::ADDI(0x11, REG_ZERO, 0x2a),
        Opcode::ADD(0x12, 0x10, 0x11),
        Opcode::LOG(0x10, 0x11, 0x12, 0x00),
        Opcode::RET(0x12),
    ]
    .iter()
    .copied()
    .collect();

    let salt: [u8; 32] = rng.gen();

    let compiled_contract = CompiledContract {
        salt: Salt::from(salt),
        raw: program,
        inputs: vec![],
        outputs: vec![],
        target_network_url: constants::DEFAULT_NODE_URL.to_string(),
    };

    let (_, contract_id) = Contract::launch_and_deploy(&compiled_contract, true)
        .await
        .unwrap();

    // Call contract
    let utxo_id = Bytes32::from([0u8; 32]);
    let balance_root = Bytes32::from([0u8; 32]);
    let state_root = Bytes32::from([0u8; 32]);
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let input_index = 0;

    let receipts = Contract::call(
        contract_id,
        utxo_id,
        balance_root,
        state_root,
        input_index,
        gas_price,
        gas_limit,
        maturity,
    )
    .await
    .unwrap();

    assert_eq!(true, receipts.len() > 0);

    // Grab the receipt of type `Log`
    let receipt = receipts[1];

    assert_eq!(receipt.ra().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipt.rb().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipt.rc().expect("Receipt value failed"), 0x54);
}
