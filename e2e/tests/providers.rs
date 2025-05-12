use std::{ops::Add, path::Path};

use chrono::{DateTime, Duration, TimeZone, Utc};
use fuel_asm::RegId;
use fuel_tx::Witness;
use fuels::{
    accounts::{
        Account,
        signers::{fake::FakeSigner, private_key::PrivateKeySigner},
    },
    client::{PageDirection, PaginationRequest},
    prelude::*,
    tx::Receipt,
    types::{
        Bits256,
        coin_type::CoinType,
        message::Message,
        transaction_builders::{BuildableTransaction, ScriptTransactionBuilder},
        tx_status::{Success, TxStatus},
    },
};
use futures::StreamExt;
use rand::thread_rng;

#[tokio::test]
async fn test_provider_launch_and_connect() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let signer = PrivateKeySigner::random(&mut thread_rng());

    let coins = setup_single_asset_coins(
        signer.address(),
        AssetId::zeroed(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let provider = setup_test_provider(coins, vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let contract_instance_connected = MyContract::new(contract_id.clone(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42)
        .call()
        .await?;
    assert_eq!(42, response.value);

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
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let node_config = NodeConfig::default();
    let chain_config = ChainConfig::default();
    let state_config = StateConfig::default();
    let service = FuelService::start(node_config, chain_config, state_config).await?;
    let provider = Provider::connect(service.bound_address().to_string()).await?;

    let wallet = Wallet::random(&mut thread_rng(), provider.clone());

    // Simulate an unreachable node
    service.stop().await.unwrap();

    let response = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
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

    let signer = PrivateKeySigner::random(&mut thread_rng());

    // coin to pay transaction fee
    let coins =
        setup_single_asset_coins(signer.address(), AssetId::zeroed(), 1, DEFAULT_COIN_AMOUNT);

    let messages = vec![setup_single_message(
        &Bech32Address::default(),
        signer.address(),
        DEFAULT_COIN_AMOUNT,
        0.into(),
        vec![1, 2],
    )];

    let provider = setup_test_provider(coins, messages.clone(), None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
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
    let signer = PrivateKeySigner::random(&mut thread_rng());

    let messages = setup_single_message(
        &Bech32Address {
            hrp: "".to_string(),
            hash: Default::default(),
        },
        signer.address(),
        DEFAULT_COIN_AMOUNT,
        0.into(),
        vec![],
    );

    let provider = setup_test_provider(vec![], vec![messages], None, None).await?;
    let consensus_parameters = provider.consensus_parameters().await?;
    let base_asset_id = consensus_parameters.base_asset_id();
    let wallet = Wallet::new(signer, provider.clone());

    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let deploy_response = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?;

    let contract_instance = MyContract::new(deploy_response.contract_id, wallet.clone());

    let call_response = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?;

    assert_eq!(42, call_response.value);

    let balance = wallet.get_asset_balance(base_asset_id).await?;
    let deploy_fee = deploy_response.tx_status.unwrap().total_fee;
    let call_fee = call_response.tx_status.total_fee;
    assert_eq!(balance, DEFAULT_COIN_AMOUNT - deploy_fee - call_fee);

    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<()> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await?;
    let wallet = &wallets[0];
    let provider = wallet.provider();

    assert_eq!(provider.latest_block_height().await?, 0u32);

    provider.produce_blocks(3, None).await?;

    assert_eq!(provider.latest_block_height().await?, 3u32);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

// debug builds are slower (20x for `fuel-core-lib`, 4x for a release-fuel-core-binary), makes for
// flaky tests
#[cfg(not(feature = "fuel-core-lib"))]
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
    let provider = wallet.provider();

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
    let blocks: Vec<fuels::types::block::Block> = provider.get_blocks(req).await?.results;

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
async fn contract_deployment_respects_maturity_and_expiration() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/transaction_block_height/out/release/transaction_block_height-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.provider().clone();

    let maturity = 10;
    let expiration = 20;

    let deploy_w_maturity_and_expiration = || {
        Contract::load_from(
            "sway/contracts/transaction_block_height/out/release/transaction_block_height.bin",
            LoadConfiguration::default(),
        )
        .map(|loaded_contract| {
            loaded_contract.deploy(
                &wallet,
                TxPolicies::default()
                    .with_maturity(maturity)
                    .with_expiration(expiration),
            )
        })
    };

    {
        let err = deploy_w_maturity_and_expiration()?
            .await
            .expect_err("maturity not reached");

        assert!(err.to_string().contains("TransactionMaturity"));
    }
    {
        provider.produce_blocks(15, None).await?;
        deploy_w_maturity_and_expiration()?
            .await
            .expect("should succeed. Block height between `maturity` and `expiration`");
    }
    {
        provider.produce_blocks(15, None).await?;
        let err = deploy_w_maturity_and_expiration()?
            .await
            .expect_err("expiration reached");

        assert!(err.to_string().contains("TransactionExpiration"));
    }

    Ok(())
}

#[tokio::test]
async fn test_gas_forwarded_defaults_to_tx_limit() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    // The gas used by the script to call a contract and forward remaining gas limit.
    let gas_used_by_script = 247;
    let gas_limit = 225_883;
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(gas_limit))
        .call()
        .await?;

    let gas_forwarded = response
        .tx_status
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
            project = "e2e/sway/contracts/token_ops"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TokenContract",
            wallet = "wallet",
            random_salt = false,
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
        .tx_status
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
        .transfer(1_000_000, asset_id, address.into())
        .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
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
        .tx_status
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
    let signer = PrivateKeySigner::random(&mut thread_rng());
    let number_of_coins = 1;
    let amount_per_coin = 1_000_000;
    let coins = setup_single_asset_coins(
        signer.address(),
        AssetId::zeroed(),
        number_of_coins,
        amount_per_coin,
    );

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    // Test running out of gas. Gas price as `None` will be 0.
    let gas_limit = 42;
    let contract_instance_call = contract_instance
        .methods()
        .initialize_counter(42) // Build the ABI call
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(gas_limit));

    //  Test that the call will use more gas than the gas limit
    let total_gas = contract_instance_call
        .estimate_transaction_cost(None, None)
        .await?
        .total_gas;
    assert!(total_gas > gas_limit);

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

    let expected = "Response errors; Validity(InsufficientFeeAmount";
    assert!(response.to_string().contains(expected));

    Ok(())
}

#[tokio::test]
async fn test_call_param_gas_errors() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
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
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let total_gas = contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?
        .tx_status
        .total_gas;

    assert!(total_gas > 0);

    Ok(())
}

#[tokio::test]
async fn test_parse_block_time() -> Result<()> {
    let signer = PrivateKeySigner::random(&mut thread_rng());
    let asset_id = AssetId::zeroed();
    let coins = setup_single_asset_coins(signer.address(), asset_id, 1, DEFAULT_COIN_AMOUNT);
    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());
    let tx_policies = TxPolicies::default().with_script_gas_limit(2000);

    let wallet_2 = wallet.lock();
    let tx_response = wallet
        .transfer(wallet_2.address(), 100, asset_id, tx_policies)
        .await?;

    let tx_response = wallet
        .try_provider()?
        .get_transaction_by_id(&tx_response.tx_id)
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

    let signer = PrivateKeySigner::random(&mut thread_rng());
    let address = signer.address();

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

    let wallet = Wallet::new(signer, provider.clone());

    let requested_amount = coin_amount_1 + coin_amount_2 + message_amount;
    let consensus_parameters = provider.consensus_parameters().await?;
    {
        let resources = wallet
            .get_spendable_resources(
                *consensus_parameters.base_asset_id(),
                requested_amount.into(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(resources.len(), 3);
    }

    {
        let filter = ResourceFilter {
            from: wallet.address().clone(),
            amount: coin_amount_1.into(),
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
    let provider = wallet.provider();

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/block_timestamp"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
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
    wallet: &Wallet,
    amount: u64,
    to: &Bech32Address,
) -> Result<ScriptTransaction> {
    let asset_id = AssetId::zeroed();
    let inputs = wallet
        .get_asset_inputs_for_amount(asset_id, amount.into(), None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(to, asset_id, amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());

    wallet.adjust_for_fee(&mut tb, amount.into()).await?;
    wallet.add_witnesses(&mut tb)?;

    tb.build(wallet.provider()).await
}

#[cfg(feature = "coin-cache")]
#[tokio::test]
async fn transactions_with_the_same_utxo() -> Result<()> {
    use fuels::types::errors::transaction;

    let wallet_1 = launch_provider_and_get_wallet().await?;
    let provider = wallet_1.provider();
    let wallet_2 = Wallet::random(&mut thread_rng(), provider.clone());

    let tx_1 = create_transfer(&wallet_1, 100, wallet_2.address()).await?;
    let tx_2 = create_transfer(&wallet_1, 101, wallet_2.address()).await?;

    let _tx_id = provider.send_transaction(tx_1).await?;
    let res = provider.send_transaction(tx_2).await;

    let err = res.expect_err("is error");

    assert!(matches!(
        err,
        Error::Transaction(transaction::Reason::Validation(..))
    ));
    assert!(
        err.to_string()
            .contains("was submitted recently in a transaction ")
    );

    Ok(())
}

#[cfg(feature = "coin-cache")]
#[tokio::test]
async fn coin_caching() -> Result<()> {
    let amount = 1000;
    let num_coins = 50;
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(1), Some(num_coins), Some(amount)),
        Some(NodeConfig::default()),
        None,
    )
    .await?;
    let wallet_1 = wallets.pop().unwrap();
    let provider = wallet_1.provider();
    let wallet_2 = Wallet::random(&mut thread_rng(), provider.clone());

    // Consecutively send transfer txs. Without caching, the txs will
    // end up trying to use the same input coins because 'get_spendable_coins()'
    // won't filter out recently used coins.
    let num_iterations = 10;
    let amount_to_send = 100;
    let mut tx_ids = vec![];
    for _ in 0..num_iterations {
        let tx = create_transfer(&wallet_1, amount_to_send, wallet_2.address()).await?;
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
    assert_eq!(
        wallet_2.get_asset_balance(&AssetId::zeroed()).await?,
        num_iterations * amount_to_send
    );

    Ok(())
}

#[cfg(feature = "coin-cache")]
async fn create_revert_tx(wallet: &Wallet) -> Result<ScriptTransaction> {
    let script = std::fs::read("sway/scripts/reverting/out/release/reverting.bin")?;

    let amount = 1u64;
    let asset_id = AssetId::zeroed();
    let inputs = wallet
        .get_asset_inputs_for_amount(asset_id, amount.into(), None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(&Bech32Address::default(), asset_id, amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default())
        .with_script(script);
    wallet.adjust_for_fee(&mut tb, amount.into()).await?;
    wallet.add_witnesses(&mut tb)?;

    tb.build(wallet.provider()).await
}

#[cfg(feature = "coin-cache")]
#[tokio::test]
async fn test_cache_invalidation_on_await() -> Result<()> {
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

    let provider = wallet.provider();
    let tx = create_revert_tx(&wallet).await?;

    // Pause time so that the cache doesn't invalidate items based on TTL
    tokio::time::pause();

    // tx inputs should be cached and then invalidated due to the tx failing
    let tx_status = provider.send_transaction_and_await_commit(tx).await?;

    assert!(matches!(tx_status, TxStatus::Failure { .. }));

    let consensus_parameters = provider.consensus_parameters().await?;
    let coins = wallet
        .get_spendable_resources(*consensus_parameters.base_asset_id(), 1, None)
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
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let provider = wallet.provider();

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
    let provider = wallet.provider();

    let receiver = Wallet::random(&mut thread_rng(), provider.clone());

    let consensus_parameters = provider.consensus_parameters().await?;
    let inputs = wallet
        .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), 100, None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        receiver.address(),
        *consensus_parameters.base_asset_id(),
        100,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(provider).await?;

    provider.send_transaction_and_await_commit(tx).await?;

    let receiver_balance = receiver
        .get_asset_balance(consensus_parameters.base_asset_id())
        .await?;

    assert_eq!(receiver_balance, 100);

    Ok(())
}

#[tokio::test]
async fn send_transaction_and_await_status() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.provider();

    let consensus_parameters = provider.consensus_parameters().await?;
    let inputs = wallet
        .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), 100, None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        &Bech32Address::default(),
        *consensus_parameters.base_asset_id(),
        100,
    );

    // Given
    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(provider).await?;

    // When
    let status = provider.send_transaction_and_await_status(tx, true).await?;

    // Then
    assert_eq!(status.len(), 3);
    assert!(status.iter().enumerate().all(|(i, tx_status)| {
        matches!(
            (i, tx_status.clone().unwrap()),
            (0, TxStatus::Submitted { .. })
                | (1, TxStatus::PreconfirmationSuccess { .. })
                | (2, TxStatus::Success { .. })
        )
    }));
    Ok(())
}

#[tokio::test]
async fn send_transaction_and_subscribe_status() -> Result<()> {
    let config = NodeConfig {
        block_production: Trigger::Never,
        ..NodeConfig::default()
    };
    let wallet =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config), None)
            .await?[0]
            .clone();
    let provider = wallet.provider();

    let consensus_parameters = provider.consensus_parameters().await?;
    let inputs = wallet
        .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), 100, None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        &Bech32Address::default(),
        *consensus_parameters.base_asset_id(),
        100,
    );

    // Given
    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(provider).await?;
    let tx_id = tx.id(consensus_parameters.chain_id());

    // When
    let mut statuses = provider.subscribe_transaction_status(&tx_id, true).await?;
    let _ = provider.send_transaction(tx).await?;

    // Then
    assert!(matches!(
        statuses.next().await.unwrap()?,
        TxStatus::Submitted { .. }
    ));
    provider.produce_blocks(1, None).await?;
    assert!(matches!(
        statuses.next().await.unwrap()?,
        TxStatus::PreconfirmationSuccess { .. }
    ));
    assert!(matches!(
        statuses.next().await.unwrap()?,
        TxStatus::Success { .. }
    ));

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
    let provider = wallet.provider();

    let consensus_parameters = provider.consensus_parameters().await?;
    let inputs = wallet
        .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), 100, None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        &Bech32Address::default(),
        *consensus_parameters.base_asset_id(),
        100,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    wallet.add_witnesses(&mut tb)?;
    let tx = tb.build(provider).await?;
    let tx_id = tx.id(consensus_parameters.chain_id());

    provider.send_transaction(tx).await?;
    provider.produce_blocks(1, None).await?;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let status = provider.tx_status(&tx_id).await?;
    assert!(matches!(status, TxStatus::Success { .. }));

    Ok(())
}

#[tokio::test]
async fn can_upload_executor_and_trigger_upgrade() -> Result<()> {
    let signer = PrivateKeySigner::random(&mut thread_rng());

    // Need more coins to avoid "not enough coins to fit the target"
    let num_coins = 100;
    let coins = setup_single_asset_coins(
        signer.address(),
        AssetId::zeroed(),
        num_coins,
        DEFAULT_COIN_AMOUNT,
    );

    let mut chain_config = ChainConfig::local_testnet();
    chain_config
        .consensus_parameters
        .set_privileged_address(signer.address().into());

    let provider = setup_test_provider(coins, vec![], None, Some(chain_config)).await?;
    let wallet = Wallet::new(signer, provider.clone());

    // This is downloaded over in `build.rs`
    let executor = std::fs::read(Path::new(env!("OUT_DIR")).join("fuel-core-wasm-executor.wasm"))?;

    let subsection_size = 65536;
    let subsections = UploadSubsection::split_bytecode(&executor, subsection_size).unwrap();

    let root = subsections[0].root;
    for subsection in subsections {
        let mut builder =
            UploadTransactionBuilder::prepare_subsection_upload(subsection, TxPolicies::default());
        wallet.add_witnesses(&mut builder)?;
        wallet.adjust_for_fee(&mut builder, 0).await?;
        let tx = builder.build(&provider).await?;

        provider.send_transaction_and_await_commit(tx).await?;
    }

    let mut builder =
        UpgradeTransactionBuilder::prepare_state_transition_upgrade(root, TxPolicies::default());
    wallet.add_witnesses(&mut builder)?;
    wallet.adjust_for_fee(&mut builder, 0).await?;
    let tx = builder.build(provider.clone()).await?;

    provider.send_transaction(tx).await?;

    Ok(())
}

#[tokio::test]
async fn tx_respects_policies() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let tip = 22;
    let witness_limit = 1000;
    let maturity = 4;
    let expiration = 128;
    let max_fee = 10_000;
    let script_gas_limit = 3000;
    let tx_policies = TxPolicies::new(
        Some(tip),
        Some(witness_limit),
        Some(maturity),
        Some(expiration),
        Some(max_fee),
        Some(script_gas_limit),
    );

    // advance the block height to ensure the maturity is respected
    let provider = wallet.provider();
    provider.produce_blocks(4, None).await?;

    // trigger a transaction that contains script code to verify
    // that policies precede estimated values
    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    let tx_response = provider
        .get_transaction_by_id(&response.tx_id.unwrap())
        .await?
        .expect("tx should exist");
    let script = match tx_response.transaction {
        TransactionType::Script(tx) => tx,
        _ => panic!("expected script transaction"),
    };

    assert_eq!(script.maturity().unwrap(), maturity);
    assert_eq!(script.expiration().unwrap(), expiration);
    assert_eq!(script.tip().unwrap(), tip);
    assert_eq!(script.witness_limit().unwrap(), witness_limit);
    assert_eq!(script.max_fee().unwrap(), max_fee);
    assert_eq!(script.gas_limit(), script_gas_limit);

    Ok(())
}

#[tokio::test]
#[ignore] // TODO: https://github.com/FuelLabs/fuels-rs/issues/1581
async fn can_setup_static_gas_price() -> Result<()> {
    let expected_gas_price = 474;
    let node_config = NodeConfig {
        starting_gas_price: expected_gas_price,
        ..Default::default()
    };
    let provider = setup_test_provider(vec![], vec![], Some(node_config), None).await?;

    let gas_price = provider.estimate_gas_price(0).await?.gas_price;

    let da_cost = 1000;
    assert_eq!(gas_price, da_cost + expected_gas_price);

    Ok(())
}

#[tokio::test]
async fn tx_with_witness_data() -> Result<()> {
    use fuel_asm::{GTFArgs, op};

    let wallet = launch_provider_and_get_wallet().await?;
    let provider = wallet.provider();

    let receiver = Wallet::random(&mut thread_rng(), provider.clone());

    let consensus_parameters = provider.consensus_parameters().await?;
    let inputs = wallet
        .get_asset_inputs_for_amount(*consensus_parameters.base_asset_id(), 10000, None)
        .await?;
    let outputs = wallet.get_asset_outputs_for_amount(
        receiver.address(),
        *consensus_parameters.base_asset_id(),
        1,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    wallet.add_witnesses(&mut tb)?;

    // we test that the witness data wasn't tempered with during the build (gas estimation) process
    // if the witness data is tempered with, the estimation will be off and the transaction
    // will error out with `OutOfGas`
    let script: Vec<u8> = vec![
        // load witness data into register 0x10
        op::gtf(0x10, 0x00, GTFArgs::WitnessData.into()),
        op::lw(0x10, 0x10, 0x00),
        // load expected value into register 0x11
        op::movi(0x11, 0x0f),
        // load the offset of the revert instruction into register 0x12
        op::movi(0x12, 0x08),
        // compare the two values and jump to the revert instruction if they are not equal
        op::jne(0x10, 0x11, 0x12),
        // do some expensive operation so gas estimation is higher if comparison passes
        op::gtf(0x13, 0x01, GTFArgs::WitnessData.into()),
        op::gtf(0x14, 0x01, GTFArgs::WitnessDataLength.into()),
        op::aloc(0x14),
        op::eck1(RegId::HP, 0x13, 0x13),
        // return the witness data
        op::ret(0x10),
        op::rvrt(RegId::ZERO),
    ]
    .into_iter()
    .collect();
    tb.script = script;

    let expected_data = 15u64;
    let witness = Witness::from(expected_data.to_be_bytes().to_vec());
    tb.witnesses_mut().push(witness);

    let tx = tb
        .with_tx_policies(TxPolicies::default().with_witness_limit(1000))
        .build(provider)
        .await?;

    let status = provider.send_transaction_and_await_commit(tx).await?;

    match status {
        TxStatus::Success(Success { receipts, .. }) => {
            let ret: u64 = receipts
                .into_iter()
                .find_map(|receipt| match receipt {
                    Receipt::Return { val, .. } => Some(val),
                    _ => None,
                })
                .expect("should have return value");

            assert_eq!(ret, expected_data);
        }
        _ => panic!("expected success status"),
    }

    Ok(())
}

#[tokio::test]
async fn contract_call_with_impersonation() -> Result<()> {
    let provider_config = NodeConfig {
        utxo_validation: false,
        ..NodeConfig::default()
    };
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(1), Some(10), Some(1000)),
        Some(provider_config),
        None,
    )
    .await?;
    let wallet = wallets.pop().unwrap();
    let provider = wallet.provider();

    let impersonator = Wallet::new(FakeSigner::new(wallet.address().clone()), provider.clone());

    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let contract_instance = MyContract::new(contract_id, impersonator.clone());

    // The gas used by the script to call a contract and forward remaining gas limit.
    contract_instance
        .methods()
        .initialize_counter(42)
        .call()
        .await?;

    Ok(())
}

#[tokio::test]
async fn is_account_query_test() -> Result<()> {
    {
        let wallet = launch_provider_and_get_wallet().await?;
        let provider = wallet.provider().clone();

        let blob = Blob::new(vec![1; 100]);
        let blob_id = blob.id();

        let is_account = provider.is_user_account(blob_id).await?;
        assert!(is_account);

        let mut tb = BlobTransactionBuilder::default().with_blob(blob);
        wallet.adjust_for_fee(&mut tb, 0).await?;
        wallet.add_witnesses(&mut tb)?;
        let tx = tb.build(provider.clone()).await?;

        provider
            .send_transaction_and_await_commit(tx)
            .await?
            .check(None)?;

        let is_account = provider.is_user_account(blob_id).await?;
        assert!(!is_account);
    }
    {
        let wallet = launch_provider_and_get_wallet().await?;
        let provider = wallet.provider().clone();

        let contract = Contract::load_from(
            "sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?;
        let contract_id = contract.contract_id();

        let is_account = provider.is_user_account(*contract_id).await?;
        assert!(is_account);

        contract.deploy(&wallet, TxPolicies::default()).await?;

        let is_account = provider.is_user_account(*contract_id).await?;
        assert!(!is_account);
    }
    {
        let wallet = launch_provider_and_get_wallet().await?;
        let provider = wallet.provider().clone();

        let mut tb = ScriptTransactionBuilder::default();
        wallet.adjust_for_fee(&mut tb, 0).await?;
        wallet.add_witnesses(&mut tb)?;
        let tx = tb.build(provider.clone()).await?;

        let consensus_parameters = provider.consensus_parameters().await?;
        let tx_id = tx.id(consensus_parameters.chain_id());
        let is_account = provider.is_user_account(tx_id).await?;
        assert!(is_account);

        provider
            .send_transaction_and_await_commit(tx)
            .await?
            .check(None)?;
        let is_account = provider.is_user_account(tx_id).await?;
        assert!(!is_account);
    }

    Ok(())
}
