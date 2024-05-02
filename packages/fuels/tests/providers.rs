use std::{ops::Add, str::FromStr};

use chrono::{DateTime, Duration, TimeZone, Utc};
use fuel_core::chain_config::StateConfig;
use fuels::{
    accounts::Account,
    client::{PageDirection, PaginationRequest},
    crypto::SecretKey,
    prelude::*,
    tx::Receipt,
    types::{
        block::Block,
        coin_type::CoinType,
        errors::transaction::Reason,
        message::Message,
        transaction_builders::{BuildableTransaction, ScriptTransactionBuilder},
        tx_status::TxStatus,
        Bits256,
    },
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
        AssetId::zeroed(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let provider = setup_test_provider(coins, vec![], None, None).await?;
    wallet.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42)
        .call()
        .await?;
    assert_eq!(42, response.value);

    wallet.set_provider(provider);
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

    let node_config = NodeConfig::default();
    let chain_config = ChainConfig::default();
    let state_config = StateConfig::default();
    let service = FuelService::start(node_config, chain_config, state_config).await?;
    let provider = Provider::connect(service.bound_address().to_string()).await?;

    wallet.set_provider(provider);

    // Simulate an unreachable node
    service.stop().await.unwrap();

    let response = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await;

    assert!(matches!(response, Err(Error::Provider(_))));
    Ok(())
}

#[tokio::test]
async fn test_input_message() -> Result<()> {
    let compare_messages =
        |messages_from_provider: Vec<Message>, used_messages: Vec<Message>| -> bool {
            std::iter::zip(&used_messages, &messages_from_provider).all(|(a, b)| {
                a.sender == b.sender
                    && a.recipient == b.recipient
                    && a.nonce == b.nonce
                    && a.amount == b.amount
            })
        };

    let mut wallet = WalletUnlocked::new_random(None);

    // coin to pay transaction fee
    let coins =
        setup_single_asset_coins(wallet.address(), AssetId::zeroed(), 1, DEFAULT_COIN_AMOUNT);

    let messages = vec![setup_single_message(
        &Bech32Address::default(),
        wallet.address(),
        DEFAULT_COIN_AMOUNT,
        0.into(),
        vec![1, 2],
    )];

    let provider = setup_test_provider(coins, messages.clone(), None, None).await?;
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
        .initialize_counter(42)
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

    let provider = setup_test_provider(vec![], vec![messages], None, None).await?;
    let base_asset_id = *provider.base_asset_id();
    wallet.set_provider(provider);

    abigen!(Contract(
        name = "MyContract",
        abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
    ));

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?;

    assert_eq!(42, response.value);

    let balance = wallet.get_asset_balance(&base_asset_id).await?;
    // expect the initial amount because gas cost defaults to 0
    assert_eq!(balance, DEFAULT_COIN_AMOUNT);

    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<()> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await?;
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
    let block_time = 20u32; // seconds
    let config = NodeConfig {
        // This is how you specify the time between blocks
        block_production: Trigger::Interval {
            block_time: std::time::Duration::from_secs(block_time.into()),
        },
        ..NodeConfig::default()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None)
            .await?;
    let wallet = &wallets[0];
    let provider = wallet.try_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0u32);
    let origin_block_time = provider.latest_block_time().await?.unwrap();
    let blocks_to_produce = 3;

    provider.produce_blocks(blocks_to_produce, None).await?;
    assert_eq!(provider.latest_block_height().await?, blocks_to_produce);
    let expected_latest_block_time = origin_block_time
        .checked_add_signed(Duration::try_seconds((blocks_to_produce * block_time) as i64).unwrap())
        .unwrap();
    assert_eq!(
        provider.latest_block_time().await?.unwrap(),
        expected_latest_block_time
    );
    // ANCHOR_END: use_produce_blocks_custom_time

    let req = PaginationRequest {
        cursor: None,
        results: 10,
        direction: PageDirection::Forward,
    };
    let blocks: Vec<Block> = provider.get_blocks(req).await?.results;

    assert_eq!(blocks[1].header.time.unwrap().timestamp(), 20);
    assert_eq!(blocks[2].header.time.unwrap().timestamp(), 40);
    assert_eq!(blocks[3].header.time.unwrap().timestamp(), 60);
    Ok(())
}

#[tokio::test]
async fn can_retrieve_latest_block_time() -> Result<()> {
    let provider = setup_test_provider(vec![], vec![], None, None).await?;
    let since_epoch = 1676039910;

    let latest_timestamp = Utc.timestamp_opt(since_epoch, 0).unwrap();
    provider.produce_blocks(1, Some(latest_timestamp)).await?;

    assert_eq!(
        provider.latest_block_time().await?.unwrap(),
        latest_timestamp
    );

    Ok(())
}

#[tokio::test]
async fn contract_deployment_respects_maturity() -> Result<()> {
    abigen!(Contract(name="MyContract", abi="packages/fuels/tests/contracts/transaction_block_height/out/debug/transaction_block_height-abi.json"));

    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await?;
    let wallet = &wallets[0];
    let provider = wallet.try_provider()?;

    let deploy_w_maturity = |maturity| {
        Contract::load_from(
            "tests/contracts/transaction_block_height/out/debug/transaction_block_height.bin",
            LoadConfiguration::default(),
        )
        .map(|loaded_contract| {
            loaded_contract.deploy(wallet, TxPolicies::default().with_maturity(maturity))
        })
    };

    let err = deploy_w_maturity(1)?.await.expect_err(
        "should not deploy contract since block height `0` is less than the requested maturity `1`",
    );

    let Error::Transaction(Reason::Validation(s)) = err else {
        panic!("expected `Validation`, got: `{err}`");
    };
    assert_eq!(s, "TransactionMaturity");

    provider.produce_blocks(1, None).await?;
    deploy_w_maturity(1)?
        .await
        .expect("Should deploy contract since maturity `1` is <= than the block height `1`");

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

    // The gas used by the script to call a contract and forward remaining gas limit.
    let gas_used_by_script = 856;
    let gas_limit = 225_883;
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(gas_limit))
        .call()
        .await?;

    let gas_forwarded = response
        .receipts
        .iter()
        .find(|r| matches!(r, Receipt::Call { .. }))
        .unwrap()
        .gas()
        .unwrap();

    assert_eq!(gas_limit, gas_forwarded + gas_used_by_script);

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
    let asset_id = contract_id.asset_id(&Bits256::zeroed());

    let mut balance_response = contract_methods
        .get_balance(contract_id, asset_id)
        .call()
        .await?;
    assert_eq!(balance_response.value, 0);

    contract_methods.mint_coins(5_000_000).call().await?;

    balance_response = contract_methods
        .get_balance(contract_id, asset_id)
        .call()
        .await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_policies = TxPolicies::default().with_script_gas_limit(1_000_000);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::default().with_amount(1_000_000);

    let response = contract_methods
        .get_msg_amount()
        .with_tx_policies(tx_policies)
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
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::zeroed()
    );

    let address = wallet.address();

    // withdraw some tokens to wallet
    contract_methods
        .transfer_coins_to_output(1_000_000, asset_id, address)
        .append_variable_outputs(1)
        .call()
        .await?;

    let asset_id = AssetId::from(*contract_id.hash());
    let call_params = CallParameters::default()
        .with_amount(0)
        .with_asset_id(asset_id);
    let tx_policies = TxPolicies::default().with_script_gas_limit(1_000_000);

    let response = contract_methods
        .get_msg_amount()
        .with_tx_policies(tx_policies)
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
        AssetId::zeroed(),
        number_of_coins,
        amount_per_coin,
    );

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
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
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(gas_limit));

    //  Test that the call will use more gas than the gas limit
    let gas_used = contract_instance_call
        .estimate_transaction_cost(None, None)
        .await?
        .gas_used;
    assert!(gas_used > gas_limit);

    let response = contract_instance_call
        .call()
        .await
        .expect_err("should error");

    let expected = "transaction reverted: OutOfGas";
    assert!(response.to_string().starts_with(expected));

    // Test for insufficient base asset amount to pay for the transaction fee
    let response = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .with_tx_policies(TxPolicies::default().with_tip(100_000_000_000))
        .call()
        .await
        .expect_err("should error");

    let expected = "provider: io error: Response errors; Validity(InsufficientFeeAmount";
    assert!(response.to_string().contains(expected));

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
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(446000))
        .call_params(CallParameters::default().with_gas_forwarded(1))?
        .call()
        .await
        .expect_err("should error");

    let expected = "transaction reverted: OutOfGas";
    dbg!(&response.to_string());
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_methods
        .initialize_counter(42)
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(1))
        .call_params(CallParameters::default().with_gas_forwarded(1_000))?
        .call()
        .await
        .expect_err("should error");

    assert!(response.to_string().contains(expected));
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
#[ignore]
async fn testnet_hello_world() -> Result<()> {
    use rand::Rng;

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
    let provider = Provider::connect("beta-4.fuel.network").await.unwrap();

    // Setup the private key.
    let secret =
        SecretKey::from_str("a0447cd75accc6b71a976fd3401a1f6ce318d27ba660b0315ee6ac347bf39568")
            .unwrap();

    // Create the wallet.
    let wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));

    let mut rng = rand::thread_rng();
    let salt: [u8; 32] = rng.gen();
    let configuration = LoadConfiguration::default().with_salt(salt);

    let tx_policies = TxPolicies::default().with_script_gas_limit(2000);

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        configuration,
    )?
    .deploy(&wallet, tx_policies)
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

    let response = contract_methods
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    assert_eq!(42, response.value);

    let response = contract_methods
        .increment_counter(10)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn test_parse_block_time() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);
    let asset_id = AssetId::zeroed();
    let coins = setup_single_asset_coins(wallet.address(), asset_id, 1, DEFAULT_COIN_AMOUNT);
    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    wallet.set_provider(provider);
    let tx_policies = TxPolicies::default().with_script_gas_limit(2000);

    let wallet_2 = WalletUnlocked::new_random(None).lock();
    let (tx_id, _) = wallet
        .transfer(wallet_2.address(), 100, asset_id, tx_policies)
        .await?;

    let tx_response = wallet
        .try_provider()?
        .get_transaction_by_id(&tx_id)
        .await?
        .unwrap();
    assert!(tx_response.time.is_some());

    let block = wallet
        .try_provider()?
        .block_by_height(tx_response.block_height.unwrap())
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
        .flat_map(|amount| setup_single_asset_coins(address, AssetId::zeroed(), 1, amount))
        .collect::<Vec<_>>();

    let message_amount = 200;
    let message = given_a_message(address.clone(), message_amount);

    let coin_1_utxo_id = coins[0].utxo_id;
    let coin_2_utxo_id = coins[1].utxo_id;

    let message_nonce = message.nonce;

    let provider = setup_test_provider(coins, vec![message], None, None).await?;

    wallet.set_provider(provider.clone());

    let requested_amount = coin_amount_1 + coin_amount_2 + message_amount;
    {
        let resources = wallet
            .get_spendable_resources(*provider.base_asset_id(), requested_amount)
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

fn convert_to_datetime(timestamp: u64) -> DateTime<Utc> {
    let unix = tai64::Tai64(timestamp).to_unix();
    DateTime::from_timestamp(unix, 0).unwrap()
}

/// This test is here in addition to `can_set_custom_block_time` because even though this test
/// passed, the Sway `timestamp` function didn't take into account the block time change. This
/// was fixed and this test is here to demonstrate the fix.
#[tokio::test]
async fn test_sway_timestamp() -> Result<()> {
    let block_time = 1u32; // seconds
    let provider_config = NodeConfig {
        block_production: Trigger::Interval {
            block_time: std::time::Duration::from_secs(block_time.into()),
        },
        ..NodeConfig::default()
    };
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(1), Some(1), Some(100)),
        Some(provider_config),
        None,
    )
    .await?;
    let wallet = wallets.pop().unwrap();
    let provider = wallet.try_provider()?;

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/block_timestamp"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    let origin_timestamp = provider.latest_block_time().await?.unwrap();
    let methods = contract_instance.methods();

    let response = methods.return_timestamp().call().await?;
    let mut expected_datetime =
        origin_timestamp.add(Duration::try_seconds(block_time as i64).unwrap());
    assert_eq!(convert_to_datetime(response.value), expected_datetime);

    let blocks_to_produce = 600;
    provider.produce_blocks(blocks_to_produce, None).await?;

    let response = methods.return_timestamp().call().await?;

    // `produce_blocks` call
    expected_datetime = expected_datetime
        .add(Duration::try_seconds((block_time * blocks_to_produce) as i64).unwrap());
    // method call
    expected_datetime = expected_datetime.add(Duration::try_seconds(block_time as i64).unwrap());

    assert_eq!(convert_to_datetime(response.value), expected_datetime);
    assert_eq!(
        provider.latest_block_time().await?.unwrap(),
        expected_datetime
    );
    Ok(())
}

#[cfg(feature = "coin-cache")]
async fn create_transfer(
    wallet: &WalletUnlocked,
    amount: u64,
    to: &Bech32Address,
) -> Result<ScriptTransaction> {
    let asset_id = AssetId::zeroed();
    let inputs = wallet.get_asset_inputs_for_amount(asset_id, amount).await?;
    let outputs = wallet.get_asset_outputs_for_amount(to, asset_id, amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    tb.add_signer(wallet.clone())?;

    wallet.adjust_for_fee(&mut tb, amount).await?;

    tb.build(wallet.try_provider()?).await
}

#[cfg(feature = "coin-cache")]
#[tokio::test]
async fn test_caching() -> Result<()> {
    use fuels_core::types::tx_status::TxStatus;

    let amount = 1000;
    let num_coins = 10;
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(1), Some(num_coins), Some(amount)),
        Some(NodeConfig::default()),
        None,
    )
    .await?;
    let wallet_1 = wallets.pop().unwrap();
    let provider = wallet_1.provider().unwrap();
    let wallet_2 = WalletUnlocked::new_random(Some(provider.clone()));

    // Consecutively send transfer txs. Without caching, the txs will
    // end up trying to use the same input coins because 'get_spendable_coins()'
    // won't filter out recently used coins.
    let mut tx_ids = vec![];
    for _ in 0..10 {
        let tx = create_transfer(&wallet_1, 100, wallet_2.address()).await?;
        let tx_id = provider.send_transaction(tx).await?;
        tx_ids.push(tx_id);
    }

    provider.produce_blocks(10, None).await?;

    // Confirm all txs are settled
    for tx_id in tx_ids {
        let status = provider.tx_status(&tx_id).await?;
        assert!(matches!(status, TxStatus::Success { .. }));
    }

    // Verify the transfers were successful
    assert_eq!(wallet_2.get_asset_balance(&AssetId::zeroed()).await?, 1000);

    Ok(())
}

#[cfg(feature = "coin-cache")]
async fn create_revert_tx(wallet: &WalletUnlocked) -> Result<ScriptTransaction> {
    use fuel_core_types::fuel_asm::Opcode;

    let amount = 1;
    let asset_id = AssetId::zeroed();
    let inputs = wallet.get_asset_inputs_for_amount(asset_id, amount).await?;
    let outputs = wallet.get_asset_outputs_for_amount(&Bech32Address::default(), asset_id, amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default())
        .with_script(vec![Opcode::RVRT.into()]);
    tb.add_signer(wallet.clone())?;

    wallet.adjust_for_fee(&mut tb, amount).await?;

    tb.build(wallet.try_provider()?).await
}

#[cfg(feature = "coin-cache")]
#[tokio::test]
async fn test_cache_invalidation_on_await() -> Result<()> {
    use fuels_core::types::tx_status::TxStatus;

    let block_time = 1u32;
    let provider_config = NodeConfig {
        block_production: Trigger::Interval {
            block_time: std::time::Duration::from_secs(block_time.into()),
        },
        ..NodeConfig::default()
    };

    // create wallet with 1 coin so that the cache prevents further
    // spending unless the coin is invalidated from the cache
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(1), Some(1), Some(100)),
        Some(provider_config),
        None,
    )
    .await?;
    let wallet = wallets.pop().unwrap();

    let provider = wallet.provider().unwrap();
    let tx = create_revert_tx(&wallet).await?;

    // Pause time so that the cache doesn't invalidate items based on TTL
    tokio::time::pause();

    // tx inputs should be cached and then invalidated due to the tx failing
    let tx_status = provider.send_transaction_and_await_commit(tx).await?;

    assert!(matches!(tx_status, TxStatus::Revert { .. }));

    let coins = wallet
        .get_spendable_resources(*provider.base_asset_id(), 1)
        .await?;
    assert_eq!(coins.len(), 1);

    Ok(())
}

#[tokio::test]
async fn can_fetch_mint_transactions() -> Result<()> {
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

    let provider = wallet.try_provider()?;

    let transactions = provider
        .get_transactions(PaginationRequest {
            cursor: None,
            results: 100,
            direction: PageDirection::Forward,
        })
        .await?
        .results;

    // TODO: remove once (fuels-rs#1093)[https://github.com/FuelLabs/fuels-rs/issues/1093] is in
    // until then the type is explicitly mentioned to check that we're reexporting it through fuels
    let _: ::fuels::types::transaction::MintTransaction = transactions
        .into_iter()
        .find_map(|tx| match tx.transaction {
            TransactionType::Mint(tx) => Some(tx),
            _ => None,
        })
        .expect("Should have had at least one mint transaction");

    Ok(())
}

#[tokio::test]
async fn test_build_with_provider() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.try_provider()?;

    let receiver = WalletUnlocked::new_random(Some(provider.clone()));

    let inputs = wallet
        .get_asset_inputs_for_amount(*provider.base_asset_id(), 100)
        .await?;
    let outputs =
        wallet.get_asset_outputs_for_amount(receiver.address(), *provider.base_asset_id(), 100);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    tb.add_signer(wallet.clone())?;

    let tx = tb.build(provider).await?;

    provider.send_transaction_and_await_commit(tx).await?;

    let receiver_balance = receiver.get_asset_balance(provider.base_asset_id()).await?;

    assert_eq!(receiver_balance, 100);

    Ok(())
}

#[tokio::test]
async fn can_produce_blocks_with_trig_never() -> Result<()> {
    let config = NodeConfig {
        block_production: Trigger::Never,
        ..NodeConfig::default()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None)
            .await?;
    let wallet = &wallets[0];
    let provider = wallet.try_provider()?;

    let inputs = wallet
        .get_asset_inputs_for_amount(*provider.base_asset_id(), 100)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        &Bech32Address::default(),
        *provider.base_asset_id(),
        100,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    tb.add_signer(wallet.clone())?;
    let tx = tb.build(provider).await?;
    let tx_id = tx.id(provider.chain_id());

    provider.send_transaction(tx).await?;
    provider.produce_blocks(1, None).await?;

    let status = provider.tx_status(&tx_id).await?;
    assert!(matches!(status, TxStatus::Success { .. }));

    Ok(())
}
