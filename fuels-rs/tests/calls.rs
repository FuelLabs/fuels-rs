use fuel_asm::Opcode;
use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;
use fuel_tx::{Receipt, Salt, Transaction};
use fuel_types::ContractId;
use fuel_vm::consts::REG_ZERO;
use fuels_rs::contract::{CompiledContract, Contract};
use fuels_rs::script::Script;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[tokio::test]
async fn script_call() {
    let fuel_client = setup_local_node().await;

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

    let script = Script::new(tx);

    let result = script.call(&fuel_client).await.unwrap();

    let expected_receipt = Receipt::Return {
        id: ContractId::new([0u8; 32]),
        val: 0,
        pc: result[0].pc().unwrap(),
        is: 464,
    };

    assert_eq!(expected_receipt, result[0]);
}

#[tokio::test]
async fn contract_call() {
    let rng = &mut StdRng::seed_from_u64(2321u64);
    let fuel_client = setup_local_node().await;

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
    };

    let contract_id = Contract::deploy(&compiled_contract, &fuel_client)
        .await
        .unwrap();

    // Call contract
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let receipts = Contract::call(
        contract_id,
        None,
        None,
        &fuel_client,
        gas_price,
        gas_limit,
        maturity,
        false,
    )
    .await
    .unwrap();

    assert!(receipts.len() > 0);

    // Grab the receipt of type `Log`
    let receipt = &receipts[1];

    assert_eq!(receipt.ra().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipt.rb().expect("Receipt value failed"), 0x2a);
    assert_eq!(receipt.rc().expect("Receipt value failed"), 0x54);
}

async fn setup_local_node() -> FuelClient {
    let srv = FuelService::new_node(Config::local_node()).await.unwrap();
    FuelClient::from(srv.bound_address)
}
