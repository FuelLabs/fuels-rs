#[cfg(test)]
mod tests {
    #[cfg(feature = "fuel-core-lib")]
    use fuels::prelude::Config;
    use fuels::prelude::Error;

    #[tokio::test]
    #[cfg(feature = "fuel-core-lib")]
    async fn instantiate_client() -> Result<(), Error> {
        // ANCHOR: instantiate_client
        use fuels::client::FuelClient;
        use fuels::node::service::{Config, FuelService};

        // Run the fuel node.
        let server = FuelService::new_node(Config::local_node()).await?;

        // Create a client that will talk to the node created above.
        let client = FuelClient::from(server.bound_address);
        assert!(client.health().await?);
        // ANCHOR_END: instantiate_client
        Ok(())
    }

    #[tokio::test]
    async fn deploy_contract() -> Result<(), Error> {
        use fuels::prelude::*;

        // ANCHOR: deploy_contract
        // This will generate your contract's methods onto `MyContract`.
        // This means an instance of `MyContract` will have access to all
        // your contract's methods that are running on-chain!
        abigen!(
            MyContract,
            // This path is relative to the workspace (repository) root
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

        // This helper will launch a local node and provide a test wallet linked to it
        let wallet = launch_provider_and_get_wallet().await;

        // Optional: Configure deployment parameters or use `TxParameters::default()`
        let gas_price = 0;
        let gas_limit = 1_000_000;
        let byte_price = 0;
        let maturity = 0;

        // This will deploy your contract binary onto the chain so that its ID can
        // be used to initialize the instance
        let contract_id = Contract::deploy(
            // This path is relative to the current crate (examples/contracts)
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::new(
                Some(gas_price),
                Some(gas_limit),
                Some(byte_price),
                Some(maturity),
            ),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {:x}", contract_id);
        // ANCHOR_END: deploy_contract

        // ANCHOR: use_deployed_contract
        // This is an instance of your contract which you can use to make calls to your functions
        let contract_instance = MyContract::new(contract_id.to_string(), wallet);

        let response = contract_instance
            .initialize_counter(42) // Build the ABI call
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);

        let response = contract_instance.increment_counter(10).call().await?;

        assert_eq!(52, response.value);
        // ANCHOR_END: use_deployed_contract
        Ok(())
    }

    #[tokio::test]
    async fn deploy_with_parameters() -> Result<(), Error> {
        // ANCHOR: deploy_with_parameters
        use fuels::prelude::*;
        use rand::prelude::{Rng, SeedableRng, StdRng};

        // ANCHOR: abigen_example
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );
        // ANCHOR_END: abigen_example

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id_1 = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {:x}", contract_id_1);

        let rng = &mut StdRng::seed_from_u64(2322u64);
        let salt: [u8; 32] = rng.gen();

        let contract_id_2 = Contract::deploy_with_parameters(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
            Salt::from(salt),
        )
        .await?;

        println!("Contract deployed @ {:x}", contract_id_2);

        assert_ne!(contract_id_1, contract_id_2);
        // ANCHOR_END: deploy_with_parameters
        Ok(())
    }

    #[tokio::test]
    async fn deploy_with_multiple_wallets() -> Result<(), Error> {
        // ANCHOR: deploy_with_multiple_wallets
        use fuels::prelude::*;

        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

        let wallets = launch_custom_provider_and_get_wallets(WalletsConfig::default(), None).await;

        let contract_id_1 = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallets[0],
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {:x}", contract_id_1);
        let contract_instance_1 = MyContract::new(contract_id_1.to_string(), wallets[0].clone());

        let response = contract_instance_1
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);

        let contract_id_2 = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallets[1],
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {:x}", contract_id_2);
        let contract_instance_2 = MyContract::new(contract_id_2.to_string(), wallets[1].clone());

        let response = contract_instance_2
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);
        // ANCHOR_END: deploy_with_multiple_wallets
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn contract_tx_and_call_params() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        println!("Contract deployed @ {:x}", contract_id);
        // ANCHOR: instantiate_contract
        let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());
        // ANCHOR_END: instantiate_contract
        // ANCHOR: tx_parameters
        // In order: gas_price, gas_limit, byte_price, and maturity
        let my_tx_params = TxParameters::new(None, Some(1_000_000), None, None);

        let response = contract_instance
            .initialize_counter(42) // Our contract method.
            .tx_params(my_tx_params) // Chain the tx params setting method.
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.

        // ANCHOR_END: tx_parameters

        // ANCHOR: tx_parameters_default
        let response = contract_instance
            .initialize_counter(42)
            .tx_params(TxParameters::default())
            .call()
            .await?;

        // ANCHOR_END: tx_parameters_default
        // ANCHOR: tx_parameters
        // In order: gas_price, gas_limit, byte_price, and maturity
        let my_tx_params = TxParameters::new(None, Some(1_000_000), None, None);

        let response = contract_instance
            .initialize_counter(42) // Our contract method.
            .tx_params(my_tx_params) // Chain the tx params setting method.
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.

        // ANCHOR_END: tx_parameters

        // ANCHOR: call_parameters

        let tx_params = TxParameters::default();

        // Forward 1_000_000 coin amount of base asset_id
        // this is a big number for checking that amount can be a u64
        let call_params = CallParameters::new(Some(1_000_000), None, None);

        let response = contract_instance
            .get_msg_amount() // Our contract method.
            .tx_params(tx_params) // Chain the tx params setting method.
            .call_params(call_params) // Chain the call params setting method.
            .call() // Perform the contract call.
            .await?;
        // ANCHOR_END: call_parameters
        // ANCHOR: call_parameters_default
        let response = contract_instance
            .initialize_counter(42)
            .call_params(CallParameters::default())
            .call()
            .await?;

        // ANCHOR_END: call_parameters_default
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn token_ops_tests() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/token_ops/out/debug/token_ops-abi\
            .json"
        );

        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/token_ops/out/debug/token_ops\
        .bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        println!("Contract deployed @ {:x}", contract_id);
        let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());
        // ANCHOR: simulate
        // you would mint 100 coins if the transaction wasn't simulated
        let counter = contract_instance.mint_coins(100).simulate().await?;
        // ANCHOR_END: simulate
        let response = contract_instance.mint_coins(1_000_000).call().await?;
        // ANCHOR: variable_outputs
        let address = wallet.address();

        // withdraw some tokens to wallet
        let response = contract_instance
            .transfer_coins_to_output(1_000_000, contract_id, address)
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: variable_outputs
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn get_contract_outputs() -> Result<(), Error> {
        use fuels::prelude::Error::ContractCallError;
        use fuels::prelude::*;
        use fuels::tx::Receipt;
        abigen!(
            TestContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );
        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test\
        .bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        let contract_instance = TestContract::new(contract_id.to_string(), wallet);

        // ANCHOR: good_practice
        let response = contract_instance.increment_counter(162).call().await?;
        // ANCHOR_END: good_practice
        // ANCHOR: contract_receipts
        let response = contract_instance.increment_counter(162).call().await;
        match response {
            // The transaction is valid and executes to completion
            Ok(call_response) => {
                let logs: Vec<String> = call_response.logs;
                let receipts: Vec<Receipt> = call_response.receipts;
                // Do things with logs and receipts
            }

            // The transaction is invalid or node is offline
            // OR
            // The transaction is valid but reverts
            Err(ContractCallError(reason, receipts)) => {
                println!("ContractCall failed with reason: {}", reason);
                println!("Transaction receipts are: {:?}", receipts);
            }
            Err(_) => {}
        }
        // ANCHOR_END: contract_receipts
        // ANCHOR: deployed_contracts
        // Replace with your contract ABI.json path
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );
        let wallet = launch_provider_and_get_wallet().await;
        // Your contract ID as a String.
        let contract_id =
            "0x068fe90ddc43b18a8f76756ecad8bf30eb0ceea33d2e6990c0185d01b0dbb675".to_string();

        let connected_contract_instance = MyContract::new(contract_id, wallet);
        // You can now use the `connected_contract_instance` just as you did above!
        // ANCHOR_END: deployed_contracts
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn call_params_gas() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

        // ANCHOR: call_params_gas
        // Set the transaction `gas_limit` to 1000 and `gas_forwarded` to 200 to specify that the
        // contract call transaction may consume up to 1000 gas, while the actual call may only use 200
        // gas
        let tx_params = TxParameters::new(None, Some(1000), None, None);
        let call_params = CallParameters::new(None, None, Some(200));

        let response = contract_instance
            .get_msg_amount() // Our contract method.
            .tx_params(tx_params) // Chain the tx params setting method.
            .call_params(call_params) // Chain the call params setting method.
            .call() // Perform the contract call.
            .await?;
        // ANCHOR_END: call_params_gas
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn multi_call_example() -> Result<(), Error> {
        use fuels::prelude::*;

        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        // ANCHOR: multi_call_prepare
        let contract_instance = MyContract::new(contract_id.to_string(), wallet.clone());

        let call_handler_1 = contract_instance.initialize_counter(42);
        let call_handler_2 = contract_instance.get_array([42; 2].to_vec());
        // ANCHOR_END: multi_call_prepare

        // ANCHOR: multi_call_build
        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);
        // ANCHOR_END: multi_call_build

        // ANCHOR: multi_call_values
        let (counter, array): (u64, Vec<u64>) = multi_call_handler.call().await?.value;
        // ANCHOR_END: multi_call_values

        // ANCHOR: multi_call_response
        let response = multi_call_handler.call::<(u64, Vec<u64>)>().await?;
        // ANCHOR_END: multi_call_response

        assert_eq!(counter, 42);
        assert_eq!(array, [42; 2]);

        Ok(())
    }
}
