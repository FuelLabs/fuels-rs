#[cfg(test)]
mod tests {
    use fuels::accounts::wallet::WalletUnlocked;
    use fuels::types::errors::{error, Error, Result};

    #[tokio::test]
    async fn instantiate_client() -> Result<()> {
        // ANCHOR: instantiate_client
        use fuels::{
            client::FuelClient,
            fuel_node::{Config, FuelService},
        };

        // Run the fuel node.
        let server = FuelService::new_node(Config::local_node())
            .await
            .map_err(|err| error!(InfrastructureError, "{err}"))?;

        // Create a client that will talk to the node created above.
        let client = FuelClient::from(server.bound_address);
        assert!(client.health().await?);
        // ANCHOR_END: instantiate_client
        Ok(())
    }

    #[tokio::test]
    async fn deploy_contract() -> Result<()> {
        use fuels::prelude::*;

        // ANCHOR: deploy_contract
        // This helper will launch a local node and provide a test wallet linked to it
        let wallet = launch_provider_and_get_wallet().await;

        // This will load and deploy your contract binary to the chain so that its ID can
        // be used to initialize the instance
        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
        .await?;

        println!("Contract deployed @ {contract_id}");
        // ANCHOR_END: deploy_contract

        Ok(())
    }

    #[tokio::test]
    async fn setup_program_test_example() -> Result<()> {
        use fuels::prelude::*;

        // ANCHOR: deploy_contract_setup_macro_short
        setup_program_test!(
            Wallets("wallet"),
            Abigen(Contract(
                name = "TestContract",
                project = "packages/fuels/tests/contracts/contract_test"
            )),
            Deploy(
                name = "contract_instance",
                contract = "TestContract",
                wallet = "wallet"
            ),
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
    async fn contract_call_cost_estimation() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
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

        assert_eq!(transaction_cost.gas_used, 625);

        Ok(())
    }

    #[tokio::test]
    async fn deploy_with_parameters() -> std::result::Result<(), Box<dyn std::error::Error>> {
        use fuels::{
            prelude::*,
            tx::{Bytes32, StorageSlot},
        };
        use rand::prelude::{Rng, SeedableRng, StdRng};

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id_1 = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
        .await?;

        println!("Contract deployed @ {contract_id_1}");

        // ANCHOR: deploy_with_parameters
        // Optional: Add `Salt`
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let salt: [u8; 32] = rng.gen();

        // Optional: Configure storage
        let key = Bytes32::from([1u8; 32]);
        let value = Bytes32::from([2u8; 32]);
        let storage_slot = StorageSlot::new(key, value);
        let storage_configuration = StorageConfiguration::from(vec![storage_slot]);
        let configuration = LoadConfiguration::default()
            .set_storage_configuration(storage_configuration)
            .set_salt(salt);

        // Optional: Configure deployment parameters
        let tx_parameters = TxParameters::default()
            .set_gas_price(0)
            .set_gas_limit(1_000_000)
            .set_maturity(0);

        let contract_id_2 = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            configuration,
        )?
        .deploy(&wallet, tx_parameters)
        .await?;

        println!("Contract deployed @ {contract_id_2}");
        // ANCHOR_END: deploy_with_parameters

        assert_ne!(contract_id_1, contract_id_2);

        // ANCHOR: use_deployed_contract
        // This will generate your contract's methods onto `MyContract`.
        // This means an instance of `MyContract` will have access to all
        // your contract's methods that are running on-chain!
        // ANCHOR: abigen_example
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));
        // ANCHOR_END: abigen_example

        // This is an instance of your contract which you can use to make calls to your functions
        let contract_instance = MyContract::new(contract_id_2, wallet);

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
    async fn deploy_with_multiple_wallets() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallets =
            launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await;

        let contract_id_1 = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallets[0], TxParameters::default())
        .await?;

        println!("Contract deployed @ {contract_id_1}");
        let contract_instance_1 = MyContract::new(contract_id_1, wallets[0].clone());

        let response = contract_instance_1
            .methods()
            .initialize_counter(42)
            .tx_params(TxParameters::default().set_gas_limit(1_000_000))
            .call()
            .await?;

        assert_eq!(42, response.value);

        let contract_id_2 = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default().set_salt([1; 32]),
        )?
        .deploy(&wallets[1], TxParameters::default())
        .await?;

        println!("Contract deployed @ {contract_id_2}");
        let contract_instance_2 = MyContract::new(contract_id_2, wallets[1].clone());

        let response = contract_instance_2
            .methods()
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::default().set_gas_limit(1_000_000))
            .call()
            .await?;

        assert_eq!(42, response.value);
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn contract_tx_and_call_params() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
        .await?;

        println!("Contract deployed @ {contract_id}");
        // ANCHOR: tx_parameters
        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();

        let my_tx_parameters = TxParameters::default()
            .set_gas_price(1)
            .set_gas_limit(1_000_000)
            .set_maturity(0);

        let response = contract_methods
            .initialize_counter(42) // Our contract method.
            .tx_params(my_tx_parameters) // Chain the tx params setting method.
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

        // ANCHOR: call_parameters
        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        let tx_params = TxParameters::default();

        // Forward 1_000_000 coin amount of base asset_id
        // this is a big number for checking that amount can be a u64
        let call_params = CallParameters::default().set_amount(1_000_000);

        let response = contract_methods
            .get_msg_amount() // Our contract method.
            .tx_params(tx_params) // Chain the tx params setting method.
            .call_params(call_params)? // Chain the call params setting method.
            .call() // Perform the contract call.
            .await?;
        // ANCHOR_END: call_parameters

        // ANCHOR: call_parameters_default
        let response = contract_methods
            .initialize_counter(42)
            .call_params(CallParameters::default())?
            .call()
            .await?;
        // ANCHOR_END: call_parameters_default
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn token_ops_tests() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/token_ops/out/debug/token_ops\
        .bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
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
            .transfer_coins_to_output(1_000_000, contract_id, address)
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: variable_outputs
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn dependency_estimation() -> Result<()> {
        use fuels::prelude::*;
        abigen!(
            Contract(name="MyContract",
            abi="packages/fuels/tests/contracts/lib_contract_caller/out/debug/lib_contract_caller-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let called_contract_id: ContractId = Contract::load_from(
            "../../packages/fuels/tests/contracts/lib_contract/out/debug/lib_contract.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
        .await?
        .into();

        let bin_path = "../../packages/fuels/tests/contracts/lib_contract_caller/out/debug/lib_contract_caller.bin";
        let caller_contract_id = Contract::load_from(bin_path, LoadConfiguration::default())?
            .deploy(&wallet, TxParameters::default())
            .await?;

        let contract_methods =
            MyContract::new(caller_contract_id.clone(), wallet.clone()).methods();

        // ANCHOR: dependency_estimation_fail
        let address = wallet.address();
        let amount = 100;

        let response = contract_methods
            .increment_from_contract_then_mint(called_contract_id, amount, address)
            .call()
            .await;

        assert!(matches!(
            response,
            Err(Error::RevertTransactionError { .. })
        ));
        // ANCHOR_END: dependency_estimation_fail

        // ANCHOR: dependency_estimation_manual
        let response = contract_methods
            .increment_from_contract_then_mint(called_contract_id, amount, address)
            .append_variable_outputs(1)
            .set_contract_ids(&[called_contract_id.into()])
            .call()
            .await?;
        // ANCHOR_END: dependency_estimation_manual

        let asset_id = AssetId::from(*caller_contract_id.hash());
        let balance = wallet.get_asset_balance(&asset_id).await?;
        assert_eq!(balance, amount);

        // ANCHOR: dependency_estimation
        let response = contract_methods
            .increment_from_contract_then_mint(called_contract_id, amount, address)
            .estimate_tx_dependencies(Some(2))
            .await?
            .call()
            .await?;
        // ANCHOR_END: dependency_estimation

        let balance = wallet.get_asset_balance(&asset_id).await?;
        assert_eq!(balance, 2 * amount);

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn get_contract_outputs() -> Result<()> {
        use fuels::{prelude::*, tx::Receipt};
        {
            abigen!(Contract(
                name = "TestContract",
                abi =
                    "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
            ));
            let wallet = launch_provider_and_get_wallet().await;

            let contract_id = Contract::load_from(
                "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
                LoadConfiguration::default(),
            )?
            .deploy(&wallet, TxParameters::default())
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
                    println!("Transaction is malformed (ValidationError): {e}");
                }
                // Failed request to provider
                Err(Error::ProviderError(reason)) => {
                    println!("Provider request failed with reason: {reason}");
                }
                // The transaction is valid but reverts
                Err(Error::RevertTransactionError {
                    reason, receipts, ..
                }) => {
                    println!("ContractCall failed with reason: {reason}");
                    println!("Transaction receipts are: {receipts:?}");
                }
                Err(_) => {}
            }
        }
        {
            // ANCHOR: deployed_contracts
            abigen!(Contract(
                name = "MyContract",
                // Replace with your contract ABI.json path
                abi =
                    "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
            ));
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
            let connected_contract_instance = MyContract::new(contract_id, wallet);
            // ANCHOR_END: deployed_contracts_hex
        }

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn call_params_gas() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
        .await?;

        let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

        // ANCHOR: call_params_gas
        // Set the transaction `gas_limit` to 10_000 and `gas_forwarded` to 4300 to specify that
        // the contract call transaction may consume up to 10_000 gas, while the actual call may
        // only use 4300 gas
        let tx_params = TxParameters::default().set_gas_limit(10_000);
        let call_params = CallParameters::default().set_gas_forwarded(4300);

        let response = contract_methods
            .get_msg_amount() // Our contract method.
            .tx_params(tx_params) // Chain the tx params setting method.
            .call_params(call_params)? // Chain the call params setting method.
            .call() // Perform the contract call.
            .await?;
        // ANCHOR_END: call_params_gas
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn multi_call_example() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
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

        // ANCHOR: multi_contract_call_response
        let response = multi_call_handler.call::<(u64, [u64; 2])>().await?;
        // ANCHOR_END: multi_contract_call_response

        assert_eq!(counter, 42);
        assert_eq!(array, [42; 2]);

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn multi_call_cost_estimation() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxParameters::default())
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

        assert_eq!(transaction_cost.gas_used, 1021);

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn connect_wallet() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));
        let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
        let wallet_1 = wallets.pop().unwrap();
        let wallet_2 = wallets.pop().unwrap();

        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet_1, TxParameters::default())
        .await?;

        // ANCHOR: connect_wallet
        // Create contract instance with wallet_1
        let contract_instance = MyContract::new(contract_id, wallet_1.clone());

        // Perform contract call with wallet_2
        let response = contract_instance
            .with_account(wallet_2)? // Connect wallet_2
            .methods() // Get contract methods
            .get_msg_amount() // Our contract method
            .call() // Perform the contract call.
            .await?; // This is an async call, `.await` for it.
                     // ANCHOR_END: connect_wallet

        Ok(())
    }

    #[tokio::test]
    async fn custom_assets_example() -> Result<()> {
        use fuels::prelude::*;

        setup_program_test!(
            Wallets("wallet"),
            Abigen(Contract(
                name = "MyContract",
                project = "packages/fuels/tests/contracts/contract_test"
            )),
            Deploy(
                name = "contract_instance",
                contract = "MyContract",
                wallet = "wallet"
            )
        );

        let other_wallet = WalletUnlocked::new_random(None);

        // ANCHOR: add_custom_assets
        let amount = 1000;
        let _ = contract_instance
            .methods()
            .initialize_counter(42)
            .add_custom_asset(BASE_ASSET_ID, amount, Some(other_wallet.address().clone()))
            .call()
            .await?;
        // ANCHOR_END: add_custom_assets

        Ok(())
    }

    #[tokio::test]
    async fn low_level_call_example() -> Result<()> {
        use fuels::{
            core::codec::{calldata, fn_selector},
            prelude::*,
            types::SizedAsciiString,
        };

        setup_program_test!(
            Wallets("wallet"),
            Abigen(
                Contract(
                    name = "MyCallerContract",
                    project = "packages/fuels/tests/contracts/low_level_caller"
                ),
                Contract(
                    name = "MyTargetContract",
                    project = "packages/fuels/tests/contracts/contract_test"
                ),
            ),
            Deploy(
                name = "caller_contract_instance",
                contract = "MyCallerContract",
                wallet = "wallet"
            ),
            Deploy(
                name = "target_contract_instance",
                contract = "MyTargetContract",
                wallet = "wallet"
            ),
        );

        // ANCHOR: low_level_call
        let function_selector =
            fn_selector!(set_value_multiple_complex(MyStruct, SizedAsciiString::<4>));
        let call_data = calldata!(
            MyStruct {
                a: true,
                b: [1, 2, 3],
            },
            SizedAsciiString::<4>::try_from("fuel").unwrap()
        );

        caller_contract_instance
            .methods()
            .call_low_level_call(
                target_contract_instance.id(),
                Bytes(function_selector),
                Bytes(call_data),
                false,
            )
            .estimate_tx_dependencies(None)
            .await?
            .call()
            .await?;
        // ANCHOR_END: low_level_call

        let result_uint = target_contract_instance
            .methods()
            .get_value()
            .call()
            .await
            .unwrap()
            .value;

        let result_bool = target_contract_instance
            .methods()
            .get_bool_value()
            .call()
            .await
            .unwrap()
            .value;

        let result_str = target_contract_instance
            .methods()
            .get_str_value()
            .call()
            .await
            .unwrap()
            .value;

        assert_eq!(result_uint, 2);
        assert!(result_bool);
        assert_eq!(result_str, "fuel");

        Ok(())
    }
}
