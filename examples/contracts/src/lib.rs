#[allow(unused_imports)]
use fuels::core::errors::Error;

#[tokio::test]
#[cfg(feature = "fuel-core-lib")]
// ANCHOR: instantiate_client
async fn instantiate_client() {
    use fuels::client::FuelClient;
    use fuels::node::service::{Config, FuelService};

    let server = FuelService::new_node(Config::local_node()).await.unwrap();
    let client = FuelClient::from(server.bound_address);
    assert!(client.health().await.unwrap());
}
// ANCHOR_END: instantiate_client

#[tokio::test]
// ANCHOR: deploy_contract
async fn deploy_contract() {
    use fuels::prelude::*;

    // This will generate your contract's methods onto `MyContract`.
    // This means an instance of `MyContract` will have access to all
    // your contract's methods that are running on-chain!
    abigen!(
            MyContract,
            // This path is relative to the workspace (repository) root
            "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

    // This helper will launch a local node and provide a test wallet linked to it
    let wallet = launch_provider_and_get_single_wallet().await;

    // Optional: Configure deployment parameters or use `TxParameters::default()`
    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    // This will deploy your contract binary onto the chain so that its ID can
    // be used to initialize the instance
    let contract_id = Contract::deploy(
            // This path is relative to the current crate (examples/contracts)
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::new(
                Some(gas_price),
                Some(gas_limit),
                Some(byte_price),
                Some(maturity)
            )
        )
        .await
        .unwrap();
    println!("Contract deployed @ {:x}", contract_id);

    // Here is an instance of your contract which you can use to make calls to
    // your functions
    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let result = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
        .call() // Perform the network call
        .await
        .unwrap();

    assert_eq!(42, result.value);

    let result = contract_instance
        .increment_counter(10)
        .call()
        .await
        .unwrap();

    assert_eq!(52, result.value);
}
// ANCHOR_END: deploy_contract

#[tokio::test]
// ANCHOR: deploy_with_salt
async fn deploy_with_salt() {
    use fuels::prelude::*;
    use rand::prelude::{Rng, SeedableRng, StdRng};

    abigen!(
            MyContract,
            "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id_1 = Contract::deploy(
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
        )
        .await
        .unwrap();

    println!("Contract deployed @ {:x}", contract_id_1);

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: [u8; 32] = rng.gen();

    let contract_id_2 = Contract::deploy_with_salt(
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            Salt::from(salt),
        )
        .await
        .unwrap();

    println!("Contract deployed @ {:x}", contract_id_2);

    assert_ne!(contract_id_1, contract_id_2);
}
// ANCHOR_END: deploy_with_salt

#[tokio::test]
// ANCHOR: deploy_with_multiple_wallets
async fn deploy_with_multiple_wallets() {
    use fuels::prelude::*;

    abigen!(
            MyContract,
            "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

    let wallets = launch_provider_and_get_wallets(WalletsConfig::default()).await;

    let contract_id_1 = Contract::deploy(
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallets[0],
            TxParameters::default(),
        )
        .await
        .unwrap();

    println!("Contract deployed @ {:x}", contract_id_1);
    let contract_instance_1 = MyContract::new(contract_id_1.to_string(), wallets[0].clone());

    let result = contract_instance_1
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
        .call() // Perform the network call
        .await
        .unwrap();

    assert_eq!(42, result.value);

    let contract_id_2 = Contract::deploy(
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallets[1],
            TxParameters::default(),
        )
        .await
        .unwrap();

    println!("Contract deployed @ {:x}", contract_id_2);
    let contract_instance_2 = MyContract::new(contract_id_2.to_string(), wallets[1].clone());

    let result = contract_instance_2
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
        .call() // Perform the network call
        .await
        .unwrap();

    assert_eq!(42, result.value);
}
// ANCHOR_END: deploy_with_multiple_wallets

#[tokio::test]
async fn call_params() -> Result<(), Error> {
    use fuels::prelude::*;

    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/msg_amount/out/debug/msg_amount-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "../../packages/fuels-abigen-macro/tests/test_projects/msg_amount/out/debug/msg_amount.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

    // ANCHOR: call_params
    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);

    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = contract_instance
        .get_msg_amount() // Our contract method.
        .tx_params(tx_params) // Chain the tx params setting method.
        .call_params(call_params) // Chain the call params setting method.
        .call() // Perform the contract call.
        .await?;
    // ANCHOR_END: call_params

    print!("{:?}", response);
    Ok(())
}

#[tokio::test]
async fn call_params_gas() -> Result<(), Error> {
    use fuels::prelude::*;

    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/msg_amount/out/debug/msg_amount-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    let contract_id = Contract::deploy(
        "../../packages/fuels-abigen-macro/tests/test_projects/msg_amount/out/debug/msg_amount.bin",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

    // ANCHOR: call_params_gas
    let my_tx_params = TxParameters::new(None, Some(1000), None, None);
    let my_call_params = CallParameters::new(None, None, Some(200));

    let response = contract_instance
        .get_msg_amount() // Our contract method.
        .tx_params(my_tx_params) // Chain the tx params setting method.
        .call_params(my_call_params) // Chain the call params setting method.
        .call() // Perform the contract call.
        .await?;
    // ANCHOR_END: call_params_gas

    print!("{:?}", response);
    Ok(())
}
