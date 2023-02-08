use std::{iter, str::FromStr};

use chrono::Duration;
use fuel_core::service::{Config as CoreConfig, FuelService, ServiceTrait};
use fuels::{
    client::{PageDirection, PaginationRequest},
    prelude::*,
    signers::fuel_crypto::SecretKey,
    tx::Receipt,
    types::{block::Block, errors::error, message::Message},
};

#[tokio::test]
async fn test_provider_launch_and_connect() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let mut wallet = WalletUnlocked::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, vec![], None, None).await;
    let connected_provider = Provider::connect(address.to_string()).await?;

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    assert_eq!(42, response.value);

    wallet.set_provider(launched_provider);
    let contract_instance_launched = MyContract::new(contract_id, wallet);

    let response = contract_instance_launched
        .methods()
        .increment_counter(10)
        .call()
        .await?;
    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_network_error() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let mut wallet = WalletUnlocked::new_random(None);

    let config = CoreConfig::local_node();
    let service = FuelService::new_node(config)
        .await
        .map_err(|err| error!(InfrastructureError, "{err}"))?;
    let provider = Provider::connect(service.bound_address.to_string()).await?;

    wallet.set_provider(provider);

    // Simulate an unreachable node
    service.stop_and_await().await.unwrap();

    let response = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await;

    assert!(matches!(response, Err(Error::ProviderError(_))));
    Ok(())
}

#[tokio::test]
async fn test_input_message() -> Result<()> {
    let compare_messages =
        |messages_from_provider: Vec<Message>, used_messages: Vec<Message>| -> bool {
            iter::zip(&used_messages, &messages_from_provider).all(|(a, b)| {
                a.sender == b.sender
                    && a.recipient == b.recipient
                    && a.nonce == b.nonce
                    && a.amount == b.amount
            })
        };

    let mut wallet = WalletUnlocked::new_random(None);

    // Coin to pay transaction fee.
    let coins = setup_single_asset_coins(wallet.address(), AssetId::BASE, 1, DEFAULT_COIN_AMOUNT);

    let messages = setup_single_message(
        &Bech32Address::default(),
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0,
        vec![1, 2],
    );

    let (provider, _) = setup_test_provider(coins, messages.clone(), None, None).await;
    wallet.set_provider(provider);

    setup_contract_test!(
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

    let spendable_messages = wallet.get_messages().await?;

    assert!(compare_messages(spendable_messages, messages));

    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .call()
        .await?;

    assert_eq!(42, response.value);
    Ok(())
}

#[tokio::test]
async fn test_input_message_pays_fee() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);

    let messages = setup_single_message(
        &Bech32Address {
            hrp: "".to_string(),
            hash: Default::default(),
        },
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0,
        vec![],
    );

    let (provider, _) = setup_test_provider(vec![], messages, None, None).await;
    wallet.set_provider(provider);

    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?;

    assert_eq!(42, response.value);

    let balance = wallet.get_asset_balance(&BASE_ASSET_ID).await?;
    // expect the initial amount because gas cost defaults to 0
    assert_eq!(balance, DEFAULT_COIN_AMOUNT);

    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<()> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0);

    provider.produce_blocks(3, None).await?;

    assert_eq!(provider.latest_block_height().await?, 3);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

#[tokio::test]
async fn can_set_custom_block_time() -> Result<()> {
    use chrono::{TimeZone, Utc};

    // ANCHOR: use_produce_blocks_custom_time
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0);

    let time = TimeParameters {
        start_time: Utc.timestamp_opt(100, 0).unwrap(),
        block_time_interval: Duration::seconds(10),
    };
    provider.produce_blocks(3, Some(time)).await?;

    assert_eq!(provider.latest_block_height().await?, 3);

    let req = PaginationRequest {
        cursor: None,
        results: 10,
        direction: PageDirection::Forward,
    };
    let blocks: Vec<Block> = provider.get_blocks(req).await?.results;

    assert_eq!(blocks[1].header.time.unwrap().timestamp(), 100);
    assert_eq!(blocks[2].header.time.unwrap().timestamp(), 110);
    assert_eq!(blocks[3].header.time.unwrap().timestamp(), 120);
    // ANCHOR_END: use_produce_blocks_custom_time
    Ok(())
}

#[tokio::test]
async fn contract_deployment_respects_maturity() -> Result<()> {
    abigen!(Contract(name="MyContract", abi="packages/fuels/tests/contracts/transaction_block_height/out/debug/transaction_block_height-abi.json"));

    let config = Config {
        manual_blocks_enabled: true,
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    let deploy_w_maturity = |maturity| {
        let parameters = TxParameters {
            maturity,
            ..TxParameters::default()
        };
        Contract::deploy(
            "tests/contracts/transaction_block_height/out/debug/transaction_block_height.bin",
            wallet,
            parameters,
            StorageConfiguration::default(),
        )
    };

    let err = deploy_w_maturity(1).await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(
        err,
        Error::ValidationError(fuel_tx::CheckError::TransactionMaturity)
    ));

    provider.produce_blocks(1, None).await?;
    deploy_w_maturity(1)
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");
    Ok(())
}

#[tokio::test]
async fn test_gas_forwarded_defaults_to_tx_limit() -> Result<()> {
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

    let gas_limit = 225883;
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(gas_limit), None))
        .call()
        .await?;

    let gas_forwarded = response
        .receipts
        .iter()
        .find(|r| matches!(r, Receipt::Call { .. }))
        .unwrap()
        .gas()
        .unwrap();

    assert_eq!(gas_limit, gas_forwarded);

    Ok(())
}

#[tokio::test]
async fn test_amount_and_asset_forwarding() -> Result<()> {
    setup_contract_test!(
        Wallets("wallet"),
        Abigen(
            name = "TokenContract",
            abi = "packages/fuels/tests/contracts/token_ops"
        ),
        Deploy(
            name = "contract_instance",
            contract = "TokenContract",
            wallet = "wallet"
        ),
    );
    let contract_id = contract_instance.contract_id();
    let contract_methods = contract_instance.methods();

    let mut balance_response = contract_methods
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 0);

    contract_methods.mint_coins(5_000_000).call().await?;

    balance_response = contract_methods
        .get_balance(contract_id.into(), contract_id.into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = contract_methods
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)?
        .call()
        .await?;

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    contract_methods
        .transfer_coins_to_output(1_000_000, contract_id.into(), address.into())
        .append_variable_outputs(1)
        .call()
        .await?;

    let asset_id = AssetId::from(*contract_id.hash());
    let call_params = CallParameters::new(Some(0), Some(asset_id), None);
    let tx_params = TxParameters::new(None, Some(1_000_000), None);

    let response = contract_methods
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)?
        .call()
        .await?;

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*contract_id.hash())
    );
    Ok(())
}

#[tokio::test]
async fn test_gas_errors() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 1;
    let amount_per_coin = 1_000_000;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None, None).await;
    wallet.set_provider(provider);

    setup_contract_test!(
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

    // Test running out of gas. Gas price as `None` will be 0.
    let gas_limit = 100;
    let contract_instace_call = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(gas_limit), None));

    //  Test that the call will use more gas than the gas limit
    let gas_used = contract_instace_call
        .estimate_transaction_cost(None)
        .await?
        .gas_used;
    assert!(gas_used > gas_limit);

    let response = contract_instace_call
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));

    // Test for insufficient base asset amount to pay for the transaction fee
    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(Some(100_000_000_000), None, None))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: Response errors; not enough resources to fit the target";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_call_param_gas_errors() -> Result<()> {
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

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(3000), None))
        .call_params(CallParameters::new(None, None, Some(1)))?
        .call()
        .await
        .expect_err("should error");

    let expected = "Revert transaction error: OutOfGas";
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1), None))
        .call_params(CallParameters::new(None, None, Some(1000)))?
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_get_gas_used() -> Result<()> {
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

    let gas_used = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?
        .gas_used;

    assert!(gas_used > 0);
    Ok(())
}

#[tokio::test]
// TODO: currently skipping this test because the testnet isn't running
// the latest version of fuel-core. Once the testnet is updated, this test
// should be re-enabled.
#[ignore]
async fn testnet_hello_world() -> Result<()> {
    // Note that this test might become flaky.
    // This test depends on:
    // 1. The testnet being up and running;
    // 2. The testnet address being the same as the one in the test;
    // 3. The hardcoded wallet having enough funds to pay for the transaction.
    // This is a nice test to showcase the SDK interaction with
    // the testnet. But, if it becomes too problematic, we should remove it.
    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    // Create a provider pointing to the testnet.
    let provider = Provider::connect("node-beta-2.fuel.network").await.unwrap();

    // Setup the private key.
    let secret =
        SecretKey::from_str("a0447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568")
            .unwrap();

    // Create the wallet.
    let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

    let params = TxParameters::new(Some(1), Some(2000), None);

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        params,
        StorageConfiguration::default(),
    )
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

    let response = contract_methods
        .initialize_counter(42) // Build the ABI call
        .tx_params(params)
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);

    let response = contract_methods
        .increment_counter(10)
        .tx_params(params)
        .call()
        .await?;

    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_parse_block_time() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);
    let coins = setup_single_asset_coins(wallet.address(), AssetId::BASE, 1, DEFAULT_COIN_AMOUNT);
    let (provider, _) = setup_test_provider(coins.clone(), vec![], None, None).await;
    wallet.set_provider(provider);
    let tx_parameters = TxParameters::new(Some(1), Some(2000), None);
    let wallet_2 = WalletUnlocked::new_random(None).lock();
    let (tx_id, _) = wallet
        .transfer(wallet_2.address(), 100, BASE_ASSET_ID, tx_parameters)
        .await?;

    let tx_response = wallet
        .get_provider()
        .unwrap()
        .get_transaction_by_id(tx_id.as_str())
        .await?
        .unwrap();
    assert!(tx_response.time.is_some());

    let block = wallet
        .get_provider()
        .unwrap()
        .block(tx_response.block_id.unwrap().to_string().as_str())
        .await?
        .unwrap();
    assert!(block.header.time.is_some());

    Ok(())
}
