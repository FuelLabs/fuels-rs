use fuel_core::service::{Config as CoreConfig, FuelService};
use fuel_core_interfaces::model::Message;
use fuel_gql_client::client::schema::message::Message as OtherMessage;
use fuels::prelude::*;
use fuels_signers::fuel_crypto::SecretKey;
use std::{iter, str::FromStr};

#[tokio::test]
async fn test_provider_launch_and_connect() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = WalletUnlocked::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, vec![], None).await;
    let connected_provider = Provider::connect(address.to_string()).await?;

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.to_string(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    assert_eq!(42, response.value);

    wallet.set_provider(launched_provider);
    let contract_instance_launched = MyContract::new(contract_id.to_string(), wallet);

    let response = contract_instance_launched
        .methods()
        .increment_counter(10)
        .call()
        .await?;
    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_network_error() -> Result<(), anyhow::Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    let mut wallet = WalletUnlocked::new_random(None);

    let config = CoreConfig::local_node();
    let service = FuelService::new_node(config).await?;
    let provider = Provider::connect(service.bound_address.to_string()).await?;

    wallet.set_provider(provider);

    // Simulate an unreachable node
    service.stop().await;

    let response = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await;

    assert!(matches!(response, Err(Error::ProviderError(_))));
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_multi_asset() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_assets = 7;
    let coins_per_asset = 21;
    let amount_per_coin = 11;
    let (coins, asset_ids) = setup_multiple_assets_coins(
        wallet.address(),
        number_of_assets,
        coins_per_asset,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);
    let balances = wallet.get_balances().await?;
    assert_eq!(balances.len() as u64, number_of_assets);

    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);

        let expected_key = format!("{:#x}", asset_id);
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            coins_per_asset * amount_per_coin
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_single_asset() -> Result<(), Error> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), vec![], None).await;
    wallet.set_provider(provider);

    for (_utxo_id, coin) in coins {
        let balance = wallet.get_asset_balance(&coin.asset_id).await;
        assert_eq!(balance?, number_of_coins * amount_per_coin);
    }

    let balances = wallet.get_balances().await?;
    let expected_key = format!("{:#x}", BASE_ASSET_ID);
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&expected_key));
    assert_eq!(
        *balances.get(&expected_key).unwrap(),
        number_of_coins * amount_per_coin
    );

    Ok(())
}

#[tokio::test]
async fn test_input_message() -> Result<(), Error> {
    let compare_messages =
        |messages_from_provider: Vec<OtherMessage>, used_messages: Vec<Message>| -> bool {
            iter::zip(&used_messages, &messages_from_provider).all(|(a, b)| {
                a.sender == b.sender.0 .0
                    && a.recipient == b.recipient.0 .0
                    && a.owner == b.owner.0 .0
                    && a.nonce == b.nonce.0
                    && a.amount == b.amount.0
            })
        };

    let mut wallet = WalletUnlocked::new_random(None);

    let messages = setup_single_message(
        &Bech32Address {
            hrp: "".to_string(),
            hash: Default::default(),
        },
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0,
        vec![1, 2],
    );

    let (provider, _) = setup_test_provider(vec![], messages.clone(), None).await;
    wallet.set_provider(provider);

    setup_contract_test!(
        contract_instance,
        None,
        "packages/fuels/tests/test_projects/contract_test"
    );

    let messages_from_provider = wallet.get_messages().await?;
    assert!(compare_messages(messages_from_provider, messages));

    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .call()
        .await?;

    assert_eq!(42, response.value);

    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<(), Error> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0);

    provider.produce_blocks(3).await?;

    assert_eq!(provider.latest_block_height().await?, 3);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

#[tokio::test]
async fn contract_deployment_respects_maturity() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/transaction_block_height/out/debug/transaction_block_height-abi.json"
    );

    let config = Config {
        manual_blocks_enabled: true,
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    let deploy_w_maturity = |maturity| {
        let parameters = TxParameters {
            maturity,
            ..TxParameters::default()
        };
        Contract::deploy(
            "tests/test_projects/transaction_block_height/out/debug/transaction_block_height.bin",
            wallet,
            parameters,
            StorageConfiguration::default(),
        )
    };

    let err = deploy_w_maturity(1).await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(
        err,
        Error::ValidationError(fuel_gql_client::fuel_tx::ValidationError::TransactionMaturity)
    ));

    provider.produce_blocks(1).await?;
    deploy_w_maturity(1)
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");
    Ok(())
}

#[tokio::test]
async fn testnet_hello_world() -> Result<(), Error> {
    // Note that this test might become flaky.
    // This test depends on:
    // 1. The testnet being up and running;
    // 2. The testnet address being the same as the one in the test;
    // 3. The hardcoded wallet having enough funds to pay for the transaction.
    // This is a nice test to showcase the SDK interaction with
    // the testnet. But, if it becomes too problematic, we should remove it.
    abigen!(
        MyContract,
        "packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

    // Create a provider pointing to the testnet.
    let provider = Provider::connect("node-beta-1.fuel.network").await.unwrap();

    // Setup the private key.
    let secret =
        SecretKey::from_str("a0447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568")
            .unwrap();

    // Create the wallet.
    let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

    dbg!(wallet.address().to_string());

    let params = TxParameters::new(Some(1), Some(2000), None);

    let contract_id = Contract::deploy(
        "tests/test_projects/contract_test/out/debug/contract_test.bin",
        &wallet,
        params,
        StorageConfiguration::default(),
    )
    .await?;

    let contract_methods = MyContract::new(contract_id.to_string(), wallet.clone()).methods();

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
