use std::time::Duration;
use std::{iter, str::FromStr, vec};

use chrono::{TimeZone, Utc};
use fuel_core::service::{Config as CoreConfig, FuelService, ServiceTrait};
use fuels::{
    accounts::fuel_crypto::SecretKey,
    client::{PageDirection, PaginationRequest},
    prelude::*,
    tx::Receipt,
    types::{block::Block, errors::error, message::Message},
};
use fuels_accounts::Account;
use fuels_types::coin_type::CoinType;

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

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxParameters::default())
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42)
        .call()
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

    let response = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxParameters::default())
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

    let messages = vec![setup_single_message(
        &Bech32Address::default(),
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0.into(),
        vec![1, 2],
    )];

    let (provider, _) = setup_test_provider(coins, messages.clone(), None, None).await;
    wallet.set_provider(provider);

    setup_program_test!(
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
        0.into(),
        vec![],
    );

    let (provider, _) = setup_test_provider(vec![], vec![messages], None, None).await;
    wallet.set_provider(provider);

    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxParameters::default())
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
    let provider = wallet.try_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0u32);

    provider.produce_blocks(3, None).await?;

    assert_eq!(provider.latest_block_height().await?, 3u32);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

#[tokio::test]
async fn can_set_custom_block_time() -> Result<()> {
    // ANCHOR: use_produce_blocks_custom_time
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        block_production: Trigger::Interval {
            block_time: Duration::from_secs(10),
        },
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None).await;
    let wallet = &wallets[0];
    let provider = wallet.try_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0u32);

    provider
        .produce_blocks(3, Some(Utc.timestamp_opt(100, 0).unwrap()))
        .await?;

    assert_eq!(provider.latest_block_height().await?, 3u32);

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
async fn can_retrieve_latest_block_time() -> Result<()> {
    let provider = given_a_provider().await;
    let since_epoch = 1676039910;

    let latest_timestamp = Utc.timestamp_opt(since_epoch, 0).unwrap();
    provider.produce_blocks(1, Some(latest_timestamp)).await?;

    assert_eq!(
        provider.latest_block_time().await?.unwrap(),
        latest_timestamp
    );

    Ok(())
}

async fn given_a_provider() -> Provider {
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    setup_test_provider(vec![], vec![], Some(config), None)
        .await
        .0
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
    let provider = wallet.try_provider()?;

    let deploy_w_maturity = |maturity| {
        Contract::load_from(
            "tests/contracts/transaction_block_height/out/debug/transaction_block_height.bin",
            LoadConfiguration::default(),
        )
        .map(|loaded_contract| {
            loaded_contract.deploy(wallet, TxParameters::default().set_maturity(maturity))
        })
    };

    let err = deploy_w_maturity(1u32)?.await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(
        err,
        Error::ValidationError(fuel_tx::CheckError::TransactionMaturity)
    ));

    provider.produce_blocks(1, None).await?;
    deploy_w_maturity(1u32)?
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");
    Ok(())
}

#[tokio::test]
async fn test_gas_forwarded_defaults_to_tx_limit() -> Result<()> {
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

    let gas_limit = 225_883;
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .tx_params(TxParameters::default().set_gas_limit(gas_limit))
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
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TokenContract",
            project = "packages/fuels/tests/contracts/token_ops"
        )),
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

    let tx_params = TxParameters::default().set_gas_limit(1_000_000);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::default().set_amount(1_000_000);

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
    let call_params = CallParameters::default()
        .set_amount(0)
        .set_asset_id(asset_id);
    let tx_params = TxParameters::default().set_gas_limit(1_000_000);

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

    setup_program_test!(
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

    // Test running out of gas. Gas price as `None` will be 0.
    let gas_limit = 100;
    let contract_instance_call = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::default().set_gas_limit(gas_limit));

    //  Test that the call will use more gas than the gas limit
    let gas_used = contract_instance_call
        .estimate_transaction_cost(None)
        .await?
        .gas_used;
    assert!(gas_used > gas_limit);

    let response = contract_instance_call
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));

    // Test for insufficient base asset amount to pay for the transaction fee
    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::default().set_gas_price(100_000_000_000))
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: Response errors; not enough coins to fit the target";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_call_param_gas_errors() -> Result<()> {
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

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let contract_methods = contract_instance.methods();
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::default().set_gas_limit(3000))
        .call_params(CallParameters::default().set_gas_forwarded(1))?
        .call()
        .await
        .expect_err("should error");

    let expected = "Revert transaction error: OutOfGas";
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::default().set_gas_limit(1))
        .call_params(CallParameters::default().set_gas_forwarded(1_000))?
        .call()
        .await
        .expect_err("should error");

    let expected = "Provider error: gas_limit(";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}

#[tokio::test]
async fn test_get_gas_used() -> Result<()> {
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
/// This test will not work for as no endpoint supports the new `fuel-core` release yet
/// TODO: https://github.com/FuelLabs/fuels-rs/issues/978
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

    let tx_params = TxParameters::default().set_gas_price(1).set_gas_limit(2000);

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, tx_params)
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

    let response = contract_methods
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    assert_eq!(42, response.value);

    let response = contract_methods
        .increment_counter(10)
        .tx_params(tx_params)
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
    let tx_parameters = TxParameters::default().set_gas_price(1).set_gas_limit(2000);

    let wallet_2 = WalletUnlocked::new_random(None).lock();
    let (tx_id, _) = wallet
        .transfer(wallet_2.address(), 100, BASE_ASSET_ID, tx_parameters)
        .await?;

    let tx_response = wallet
        .try_provider()
        .unwrap()
        .get_transaction_by_id(tx_id.as_str())
        .await?
        .unwrap();
    assert!(tx_response.time.is_some());

    let block = wallet
        .try_provider()
        .unwrap()
        .block(tx_response.block_id.unwrap().to_string().as_str())
        .await?
        .unwrap();
    assert!(block.header.time.is_some());

    Ok(())
}

#[tokio::test]
async fn test_get_spendable_with_exclusion() -> Result<()> {
    let coin_amount_1 = 1000;
    let coin_amount_2 = 500;

    let mut wallet = WalletUnlocked::new_random(None);
    let address = wallet.address();

    let coins = [coin_amount_1, coin_amount_2]
        .into_iter()
        .flat_map(|amount| setup_single_asset_coins(address, BASE_ASSET_ID, 1, amount))
        .collect::<Vec<_>>();

    let message_amount = 200;
    let message = given_a_message(address.clone(), message_amount);

    let coin_1_utxo_id = coins[0].utxo_id;
    let coin_2_utxo_id = coins[1].utxo_id;

    let message_nonce = message.nonce;

    let (provider, _) = setup_test_provider(coins, vec![message], None, None).await;

    wallet.set_provider(provider.clone());

    let requested_amount = coin_amount_1 + coin_amount_2 + message_amount;
    {
        let resources = wallet
            .get_spendable_resources(BASE_ASSET_ID, requested_amount)
            .await
            .unwrap();
        assert_eq!(resources.len(), 3);
    }

    {
        let filter = ResourceFilter {
            from: wallet.address().clone(),
            amount: coin_amount_1,
            excluded_utxos: vec![coin_2_utxo_id],
            excluded_message_nonces: vec![message_nonce],
            ..Default::default()
        };
        let resources = provider.get_spendable_resources(filter).await.unwrap();

        match resources.as_slice() {
            [CoinType::Coin(coin)] => {
                assert_eq!(coin.utxo_id, coin_1_utxo_id);
            }
            _ => {
                panic!("This shouldn't happen!")
            }
        }
    }

    Ok(())
}

fn given_a_message(address: Bech32Address, message_amount: u64) -> Message {
    setup_single_message(
        &Bech32Address::default(),
        &address,
        message_amount,
        0.into(),
        vec![],
    )
}
