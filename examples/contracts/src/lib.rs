#[cfg(test)]
mod tests {
    use fuels::prelude::Error;

    #[tokio::test]
    async fn instantiate_client() -> Result<(), Error> {
        // ANCHOR: instantiate_client
        use fuels::client::FuelClient;
        use fuels::fuel_node::{Config, FuelService};

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
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        // This helper will launch a local node and provide a test wallet linked to it
        let wallet = launch_provider_and_get_wallet().await;

        // Optional: Configure deployment parameters or use `TxParameters::default()`
        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;

        // This will deploy your contract binary onto the chain so that its ID can
        // be used to initialize the instance
        let contract_id = Contract::deploy(
            // This path is relative to the current crate (examples/contracts)
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::new(Some(gas_price), Some(gas_limit), Some(maturity)),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {contract_id}");
        // ANCHOR_END: deploy_contract

        // ANCHOR: use_deployed_contract
        // This is an instance of your contract which you can use to make calls to your functions
        let contract_instance = MyContract::new(contract_id, wallet);

        let response = contract_instance
            .methods()
            .initialize_counter(42) // Build the ABI call
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);

        let response = contract_instance
            .methods()
            .increment_counter(10)
            .call()
            .await?;

        assert_eq!(52, response.value);
        // ANCHOR_END: use_deployed_contract

        Ok(())
    }

    #[tokio::test]
    async fn setup_contract_test_example() -> Result<(), Error> {
        use fuels::prelude::*;

        // ANCHOR: deploy_contract_setup_macro_short
        setup_contract_test!(
            contract_instance,
            wallet,
            "packages/fuels/tests/contracts/contract_test"
        );

        let response = contract_instance
            .methods()
            .initialize_counter(42)
            .call()
            .await?;

        assert_eq!(42, response.value);
        // ANCHOR_END: deploy_contract_setup_macro_short

        Ok(())
    }

    #[tokio::test]
    async fn contract_call_cost_estimation() -> Result<(), Error> {
        use fuels::prelude::*;

        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        // ANCHOR: contract_call_cost_estimation
        let contract_instance = MyContract::new(contract_id, wallet);

        let tolerance = 0.0;
        let transaction_cost = contract_instance
            .methods()
            .initialize_counter(42) // Build the ABI call
            .estimate_transaction_cost(Some(tolerance)) // Get estimated transaction cost
            .await?;
        // ANCHOR_END: contract_call_cost_estimation

        assert_eq!(transaction_cost.gas_used, 7146);

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
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );
        // ANCHOR_END: abigen_example

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id_1 = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {contract_id_1}");

        let rng = &mut StdRng::seed_from_u64(2322u64);
        let salt: [u8; 32] = rng.gen();

        let contract_id_2 = Contract::deploy_with_parameters(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
            Salt::from(salt),
        )
        .await?;

        println!("Contract deployed @ {contract_id_2}");

        assert_ne!(contract_id_1, contract_id_2);
        // ANCHOR_END: deploy_with_parameters
        Ok(())
    }

    #[tokio::test]
    async fn deploy_with_multiple_wallets() -> Result<(), Error> {
        use fuels::prelude::*;

        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallets =
            launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await;

        let contract_id_1 = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallets[0],
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {contract_id_1}");
        let contract_instance_1 = MyContract::new(contract_id_1, wallets[0].clone());

        let response = contract_instance_1
            .methods()
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::new(None, Some(1_000_000), None))
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);

        let contract_id_2 = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallets[1],
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        println!("Contract deployed @ {contract_id_2}");
        let contract_instance_2 = MyContract::new(contract_id_2, wallets[1].clone());

        let response = contract_instance_2
            .methods()
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::new(None, Some(1_000_000), None))
            .call() // Perform the network call
            .await?;

        assert_eq!(42, response.value);
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn contract_tx_and_call_params() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        println!("Contract deployed @ {contract_id}");
        // ANCHOR: tx_parameters
        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();

        // In order: gas_price, gas_limit, and maturity
        let my_tx_params = TxParameters::new(None, Some(1_000_000), None);

        let response = contract_methods
            .initialize_counter(42) // Our contract method.
            .tx_params(my_tx_params) // Chain the tx params setting method.
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.

        // ANCHOR_END: tx_parameters

        // ANCHOR: tx_parameters_default
        let response = contract_methods
            .initialize_counter(42)
            .tx_params(TxParameters::default())
            .call()
            .await?;

        // ANCHOR_END: tx_parameters_default
        // In order: gas_price, gas_limit, and maturity
        let my_tx_params = TxParameters::new(None, Some(1_000_000), None);

        let response = contract_methods
            .initialize_counter(42) // Our contract method.
            .tx_params(my_tx_params) // Chain the tx params setting method.
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.

        // ANCHOR: call_parameters
        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        let tx_params = TxParameters::default();

        // Forward 1_000_000 coin amount of base asset_id
        // this is a big number for checking that amount can be a u64
        let call_params = CallParameters::new(Some(1_000_000), None, None);

        let response = contract_methods
            .get_msg_amount() // Our contract method.
            .tx_params(tx_params) // Chain the tx params setting method.
            .call_params(call_params) // Chain the call params setting method.
            .call() // Perform the contract call.
            .await?;
        // ANCHOR_END: call_parameters
        // ANCHOR: call_parameters_default
        let response = contract_methods
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
            "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/token_ops/out/debug/token_ops\
        .bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        println!("Contract deployed @ {contract_id}");
        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();
        // ANCHOR: simulate
        // you would mint 100 coins if the transaction wasn't simulated
        let counter = contract_methods.mint_coins(100).simulate().await?;
        // ANCHOR_END: simulate
        let response = contract_methods.mint_coins(1_000_000).call().await?;
        // ANCHOR: variable_outputs
        let address = wallet.address();

        // withdraw some tokens to wallet
        let response = contract_methods
            .transfer_coins_to_output(1_000_000, contract_id.into(), address.into())
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: variable_outputs
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn get_contract_outputs() -> Result<(), Error> {
        use fuels::prelude::*;
        use fuels::tx::Receipt;
        abigen!(
            TestContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );
        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test\
        .bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        let contract_methods = TestContract::new(contract_id, wallet).methods();

        let response = contract_methods.increment_counter(162).call().await?;
        let response = contract_methods.increment_counter(162).call().await;
        match response {
            // The transaction is valid and executes to completion
            Ok(call_response) => {
                let receipts: Vec<Receipt> = call_response.receipts;
                // Do things with logs and receipts
            }
            // The transaction is malformed
            Err(Error::ValidationError(e)) => {
                println!("Transaction is malformed (ValidationError): {}", e);
            }
            // Failed request to provider
            Err(Error::ProviderError(reason)) => {
                println!("Provider request failed with reason: {}", reason);
            }
            // The transaction is valid but reverts
            Err(Error::RevertTransactionError(reason, receipts)) => {
                println!("ContractCall failed with reason: {}", reason);
                println!("Transaction receipts are: {:?}", receipts);
            }
            Err(_) => {}
        }
        // ANCHOR: deployed_contracts
        // Replace with your contract ABI.json path
        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );
        let wallet_original = launch_provider_and_get_wallet().await;

        let wallet = wallet_original.clone();
        // Your bech32m encoded contract ID.
        let contract_id: Bech32ContractId =
            "fuel1vkm285ypjesypw7vhdlhnty3kjxxx4efckdycqh3ttna4xvmxtfs6murwy"
                .parse()
                .expect("Invalid ID");

        let connected_contract_instance = MyContract::new(contract_id, wallet);
        // You can now use the `connected_contract_instance` just as you did above!
        // ANCHOR_END: deployed_contracts

        let wallet = wallet_original;
        // ANCHOR: deployed_contracts_hex
        let contract_id: ContractId =
            "0x65b6a3d081966040bbccbb7f79ac91b48c635729c59a4c02f15ae7da999b32d3"
                .parse()
                .expect("Invalid ID");
        let connected_contract_instance = MyContract::new(contract_id.into(), wallet);
        // ANCHOR_END: deployed_contracts_hex

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn call_params_gas() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        // ANCHOR: call_params_gas
        // Set the transaction `gas_limit` to 10000 and `gas_forwarded` to 4300 to specify that the
        // contract call transaction may consume up to 10000 gas, while the actual call may only use 4300
        // gas
        let tx_params = TxParameters::new(None, Some(10000), None);
        let call_params = CallParameters::new(None, None, Some(4300));

        let response = contract_methods
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
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        // ANCHOR: multi_call_prepare
        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        let call_handler_1 = contract_methods.initialize_counter(42);
        let call_handler_2 = contract_methods.get_array([42; 2]);
        // ANCHOR_END: multi_call_prepare

        // ANCHOR: multi_call_build
        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);
        // ANCHOR_END: multi_call_build

        // ANCHOR: multi_call_values
        let (counter, array): (u64, [u64; 2]) = multi_call_handler.call().await?.value;
        // ANCHOR_END: multi_call_values

        // ANCHOR: multi_call_response
        let response = multi_call_handler.call::<(u64, [u64; 2])>().await?;
        // ANCHOR_END: multi_call_response

        assert_eq!(counter, 42);
        assert_eq!(array, [42; 2]);

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn multi_call_cost_estimation() -> Result<(), Error> {
        use fuels::prelude::*;

        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        // ANCHOR: multi_call_cost_estimation
        let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

        let call_handler_1 = contract_methods.initialize_counter(42);
        let call_handler_2 = contract_methods.get_array([42; 2]);

        multi_call_handler
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let tolerance = 0.0;
        let transaction_cost = multi_call_handler
            .estimate_transaction_cost(Some(tolerance)) // Get estimated transaction cost
            .await?;
        // ANCHOR_END: multi_call_cost_estimation

        assert_eq!(transaction_cost.gas_used, 15176);

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn connect_wallet() -> Result<(), Error> {
        use fuels::prelude::*;
        abigen!(
            MyContract,
            "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        );

        let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));
        let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
        let wallet_1 = wallets.pop().unwrap();
        let wallet_2 = wallets.pop().unwrap();

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet_1,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        // ANCHOR: connect_wallet
        // Create contract instance with wallet_1
        let contract_instance = MyContract::new(contract_id, wallet_1.clone());

        // Perform contract call with wallet_2
        let response = contract_instance
            .with_wallet(wallet_2)? // Connect wallet_2
            .methods() // Get contract methods
            .get_msg_amount() // Our contract method
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.
                     // ANCHOR_END: connect_wallet

        Ok(())
    }
}
