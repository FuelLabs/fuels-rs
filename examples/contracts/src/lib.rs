#[cfg(test)]
mod tests {
    use fuels::types::{
        errors::{error, Error, Result},
        Bits256,
    };
    use crate::MyContractTest;

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
        // This will generate your contract's methods onto `MyContract`.
        // This means an instance of `MyContract` will have access to all
        // your contract's methods that are running on-chain!
        abigen!(Contract(
            name = "MyContract",
            // This path is relative to the workspace (repository) root
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

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
    async fn setup_contract_test_example() -> Result<()> {
        use fuels::prelude::*;

        // ANCHOR: deploy_contract_setup_macro_short
        setup_contract_test!(
            Wallets("wallet"),
            Abigen(
                name = "TestContract",
                abi = "packages/fuels/tests/contracts/contract_test"
            ),
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

        assert_eq!(transaction_cost.gas_used, 426);

        Ok(())
    }

    #[tokio::test]
    async fn deploy_with_parameters() -> Result<()> {
        // ANCHOR: deploy_with_parameters
        use fuels::prelude::*;
        use rand::prelude::{Rng, SeedableRng, StdRng};

        // ANCHOR: abigen_example
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));
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
    async fn deploy_with_multiple_wallets() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let wallets =
            launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await;

        let salt = [0; 32].into();
        let contract_id_1 = Contract::deploy_with_parameters(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallets[0],
            TxParameters::default(),
            StorageConfiguration::default(),
            salt,
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

        let salt = [1; 32].into();
        let contract_id_2 = Contract::deploy_with_parameters(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallets[1],
            TxParameters::default(),
            StorageConfiguration::default(),
            salt,
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
    async fn contract_tx_and_call_params() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

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
    async fn token_ops_tests() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
        ));

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
    async fn output_messages_test() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/token_ops/out/debug/token_ops-abi.json"
        ));

        let wallet = launch_provider_and_get_wallet().await;
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/token_ops/out/debug/token_ops\
        .bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;
        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();
        // ANCHOR: message_outputs
        let base_layer_address = Bits256([1u8; 32]);
        let amount = 1000;

        let response = contract_methods
            .send_message(base_layer_address, amount)
            .append_message_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: message_outputs

        // fails due to missing message output
        let response = contract_methods
            .send_message(base_layer_address, amount)
            .call()
            .await;
        assert!(matches!(response, Err(Error::RevertTransactionError(..))));

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
        let called_contract_id: ContractId = Contract::deploy(
            "../../packages/fuels/tests/contracts/lib_contract/out/debug/lib_contract.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?
        .into();

        let caller_contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/lib_contract_caller/out/debug/lib_contract_caller.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_methods =
            MyContract::new(caller_contract_id.clone(), wallet.clone()).methods();

        // ANCHOR: dependency_estimation_fail
        let address = wallet.address();
        let amount = 100;

        let response = contract_methods
            .increment_from_contract_then_mint(called_contract_id, amount, address.into())
            .call()
            .await;

        assert!(matches!(response, Err(Error::RevertTransactionError(..))));
        // ANCHOR_END: dependency_estimation_fail

        // ANCHOR: dependency_estimation_manual
        let response = contract_methods
            .increment_from_contract_then_mint(called_contract_id, amount, address.into())
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
            .increment_from_contract_then_mint(called_contract_id, amount, address.into())
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
            let contract_id = Contract::deploy(
                "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
                &wallet,
                TxParameters::default(),
                StorageConfiguration::default(),
            )
            .await?;

            let contract_methods = MyContractTest::new(contract_id, wallet).methods();

            let response = contract_methods.get_single(5).call().await?;
            assert_eq!(response.value, 5);

            let response = contract_methods.increment_counter(162).call().await?;
            // let response = contract_methods.increment_counter(162).call().await;
            // match response {
            //     // The transaction is valid and executes to completion
            //     Ok(call_response) => {
            //         let receipts: Vec<Receipt> = call_response.receipts;
            //         // Do things with logs and receipts
            //     }
            //     // The transaction is malformed
            //     Err(Error::ValidationError(e)) => {
            //         println!("Transaction is malformed (ValidationError): {e}");
            //     }
            //     // Failed request to provider
            //     Err(Error::ProviderError(reason)) => {
            //         println!("Provider request failed with reason: {reason}");
            //     }
            //     // The transaction is valid but reverts
            //     Err(Error::RevertTransactionError(reason, receipts)) => {
            //         println!("ContractCall failed with reason: {reason}");
            //         println!("Transaction receipts are: {receipts:?}");
            //     }
            //     Err(_) => {}
            // }
        }
        // {
        // ANCHOR: deployed_contracts
        //     abigen!(Contract(
        //         name = "MyContract",
        //         // Replace with your contract ABI.json path
        //         abi =
        //             "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        //     ));
        //     let wallet_original = launch_provider_and_get_wallet().await;
        //
        //     let wallet = wallet_original.clone();
        //     // Your bech32m encoded contract ID.
        //     let contract_id: Bech32ContractId =
        //         "fuel1vkm285ypjesypw7vhdlhnty3kjxxx4efckdycqh3ttna4xvmxtfs6murwy"
        //             .parse()
        //             .expect("Invalid ID");
        //
        //     let connected_contract_instance = MyContract::new(contract_id, wallet);
        //     // You can now use the `connected_contract_instance` just as you did above!
        //     // ANCHOR_END: deployed_contracts
        //
        //     let wallet = wallet_original;
        //     // ANCHOR: deployed_contracts_hex
        //     let contract_id: ContractId =
        //         "0x65b6a3d081966040bbccbb7f79ac91b48c635729c59a4c02f15ae7da999b32d3"
        //             .parse()
        //             .expect("Invalid ID");
        //     let connected_contract_instance = MyContract::new(contract_id.into(), wallet);
        //     // ANCHOR_END: deployed_contracts_hex
        // }

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
    async fn multi_call_example() -> Result<()> {
        use fuels::prelude::*;

        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

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

        assert_eq!(transaction_cost.gas_used, 692);

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

        setup_contract_test!(
            Wallets("wallet"),
            Abigen(
                name = "MyContract",
                abi = "packages/fuels/tests/contracts/contract_test"
            ),
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
}

#[allow(clippy::too_many_arguments)]
#[no_implicit_prelude]
pub mod abigen_bindings_test {
    #[allow(clippy::too_many_arguments)]
    #[no_implicit_prelude]
    pub mod my_contract_test_mod {
        use ::std::{
            clone::Clone,
            convert::{From, Into, TryFrom},
            format,
            iter::IntoIterator,
            iter::Iterator,
            marker::Sized,
            panic,
            string::ToString,
            vec,
        };

        #[allow(clippy::enum_variant_names)]
        #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ::fuels::macros::Parameterize,
        ::fuels::macros::Tokenizable,
        ::fuels::macros::TryFrom,
        )]
        pub enum State {
            A,
            B,
            C,
        }

        #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ::fuels::macros::Parameterize,
        ::fuels::macros::Tokenizable,
        ::fuels::macros::TryFrom,
        )]
        pub struct Person {
            pub name: ::fuels::types::SizedAsciiString<4usize>,
        }

        #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ::fuels::macros::Parameterize,
        ::fuels::macros::Tokenizable,
        ::fuels::macros::TryFrom,
        )]
        pub struct MyType {
            pub x: u64,
            pub y: u64,
        }

        pub struct MyContractTest<T> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }

        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee + ::std::clone::Clone>
        MyContractTest<T>
            where
                ::fuels::types::errors::Error: From<<T as ::fuels::signers::PayFee>::Error>,
        {
            pub fn new(contract_id: ::fuels::types::bech32::Bech32ContractId, account: T) -> Self {
                let log_decoder = ::fuels::programs::logs::LogDecoder {
                    type_lookup: ::fuels::core::utils::log_type_lookup(
                        &[],
                        ::std::option::Option::Some(contract_id.clone()),
                    ),
                };
                Self {
                    contract_id,
                    account,
                    log_decoder,
                }
            }
            pub fn contract_id(&self) -> &::fuels::types::bech32::Bech32ContractId {
                &self.contract_id
            }
            pub fn account(&self) -> T {
                self.account.clone()
            }
            pub fn with_account(&self, mut account: T) -> ::fuels::types::errors::Result<Self> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)?;
                account.set_provider(provider.clone());
                ::std::result::Result::Ok(Self {
                    contract_id: self.contract_id.clone(),
                    account,
                    log_decoder: self.log_decoder.clone(),
                })
            }
            pub async fn get_balances(
                &self,
            ) -> ::fuels::types::errors::Result<
                ::std::collections::HashMap<::std::string::String, u64>,
            > {
                ::fuels::signers::Account::get_provider(&self.account)?
                    .get_contract_balances(&self.contract_id)
                    .await
                    .map_err(::std::convert::Into::into)
            }
            pub fn methods(&self) -> MyContractTestMethods<T> {
                MyContractTestMethods {
                    contract_id: self.contract_id.clone(),
                    account: self.account.clone(),
                    log_decoder: self.log_decoder.clone(),
                }
            }
        }

        pub struct MyContractTestMethods<T> {
            contract_id: ::fuels::types::bech32::Bech32ContractId,
            account: T,
            log_decoder: ::fuels::programs::logs::LogDecoder,
        }

        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee + ::std::clone::Clone>
        MyContractTestMethods<T>
        {
            #[doc = "Calls the contract's `array_of_enums` function"]
            pub fn array_of_enums(
                &self,
                p: [self::State; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [self::State; 2usize]>
            {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(&provider, self.contract_id.clone(), &self.account, ::fuels::core::function_selector::resolve_fn_selector("array_of_enums", &[<[self::State; 2usize] as ::fuels::types::traits::Parameterize>::param_type()]), &[::fuels::types::traits::Tokenizable::into_token(p)], self.log_decoder.clone()).expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `array_of_structs` function"]
            pub fn array_of_structs(
                &self,
                p: [self::Person; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [self::Person; 2usize]>
            {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(&provider, self.contract_id.clone(), &self.account, ::fuels::core::function_selector::resolve_fn_selector("array_of_structs", &[<[self::Person; 2usize] as ::fuels::types::traits::Parameterize>::param_type()]), &[::fuels::types::traits::Tokenizable::into_token(p)], self.log_decoder.clone()).expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get` function"]
            pub fn get(
                &self,
                x: u64,
                y: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get",
                        &[
                            <u64 as ::fuels::types::traits::Parameterize>::param_type(),
                            <u64 as ::fuels::types::traits::Parameterize>::param_type(),
                        ],
                    ),
                    &[
                        ::fuels::types::traits::Tokenizable::into_token(x),
                        ::fuels::types::traits::Tokenizable::into_token(y),
                    ],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_alt` function"]
            pub fn get_alt(
                &self,
                t: self::MyType,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, self::MyType> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_alt",
                        &[<self::MyType as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(t)],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_array` function"]
            pub fn get_array(
                &self,
                p: [u64; 2usize],
            ) -> ::fuels::programs::contract::ContractCallHandler<T, [u64; 2usize]> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_array",
                        &[<[u64; 2usize] as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(p)],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_counter` function"]
            pub fn get_counter(&self) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("get_counter", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_msg_amount` function"]
            pub fn get_msg_amount(
                &self,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("get_msg_amount", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `get_single` function"]
            pub fn get_single(
                &self,
                x: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "get_single",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(x)],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `increment_counter` function"]
            pub fn increment_counter(
                &self,
                value: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "increment_counter",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `initialize_counter` function"]
            pub fn initialize_counter(
                &self,
                value: u64,
            ) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector(
                        "initialize_counter",
                        &[<u64 as ::fuels::types::traits::Parameterize>::param_type()],
                    ),
                    &[::fuels::types::traits::Tokenizable::into_token(value)],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
            #[doc = "Calls the contract's `new` function"]
            pub fn new(&self) -> ::fuels::programs::contract::ContractCallHandler<T, u64> {
                let provider = ::fuels::signers::Account::get_provider(&self.account)
                    .expect("Provider not set up");
                ::fuels::programs::contract::Contract::<T>::method_hash(
                    &provider,
                    self.contract_id.clone(),
                    &self.account,
                    ::fuels::core::function_selector::resolve_fn_selector("new", &[]),
                    &[],
                    self.log_decoder.clone(),
                )
                    .expect("method not found (this should never happen)")
            }
        }

        impl<T: ::fuels::signers::Account + ::fuels::signers::PayFee>
        ::fuels::programs::contract::SettableContract for MyContractTest<T>
        {
            fn id(&self) -> ::fuels::types::bech32::Bech32ContractId {
                self.contract_id.clone()
            }
            fn log_decoder(&self) -> ::fuels::programs::logs::LogDecoder {
                self.log_decoder.clone()
            }
        }
    }
}

pub use abigen_bindings_test::my_contract_test_mod::MyContractTest;
pub use abigen_bindings_test::my_contract_test_mod::MyContractTestMethods;
pub use abigen_bindings_test::my_contract_test_mod::MyType;
pub use abigen_bindings_test::my_contract_test_mod::Person;
pub use abigen_bindings_test::my_contract_test_mod::State;
