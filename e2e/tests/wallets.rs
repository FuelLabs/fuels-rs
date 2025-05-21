use fuels::{
    accounts::signers::private_key::PrivateKeySigner,
    prelude::*,
    types::{coin_type::CoinType, input::Input, output::Output},
};
use rand::{Rng, thread_rng};

async fn assert_address_balance(
    address: &Address,
    provider: &Provider,
    asset_id: &AssetId,
    amount: u64,
) {
    let balance = provider
        .get_asset_balance(address, asset_id)
        .await
        .expect("Could not retrieve balance");
    assert_eq!(balance, amount);
}

#[tokio::test]
async fn test_wallet_balance_api_multi_asset() -> Result<()> {
    let signer = PrivateKeySigner::random(&mut rand::thread_rng());
    let number_of_assets = 7;
    let coins_per_asset = 21;
    let amount_per_coin = 11;
    let (coins, asset_ids) = setup_multiple_assets_coins(
        signer.address(),
        number_of_assets,
        coins_per_asset,
        amount_per_coin,
    );

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());
    let balances = wallet.get_balances().await?;
    assert_eq!(balances.len() as u64, number_of_assets);

    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);

        let expected_key = asset_id.to_string();
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            (coins_per_asset * amount_per_coin) as u128
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_single_asset() -> Result<()> {
    let signer = PrivateKeySigner::random(&mut rand::thread_rng());
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        signer.address(),
        AssetId::zeroed(),
        number_of_coins,
        amount_per_coin,
    );

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    for coin in coins {
        let balance = wallet.get_asset_balance(&coin.asset_id).await;
        assert_eq!(balance?, number_of_coins * amount_per_coin);
    }

    let balances = wallet.get_balances().await?;
    let expected_key = AssetId::zeroed().to_string();
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&expected_key));
    assert_eq!(
        *balances.get(&expected_key).unwrap(),
        (number_of_coins * amount_per_coin) as u128
    );

    Ok(())
}

fn base_asset_wallet_config(num_wallets: u64) -> WalletsConfig {
    let asset_configs = vec![AssetConfig {
        id: AssetId::zeroed(),
        num_coins: 20,
        coin_amount: 20,
    }];
    WalletsConfig::new_multiple_assets(num_wallets, asset_configs)
}

#[tokio::test]
async fn adjust_fee_empty_transaction() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;

    let mut tb = ScriptTransactionBuilder::prepare_transfer(vec![], vec![], TxPolicies::default());
    assert!(tb.inputs().is_empty());
    assert!(tb.outputs().is_empty());

    wallet.add_witnesses(&mut tb)?;
    wallet.adjust_for_fee(&mut tb, 0).await?;
    assert!(!tb.inputs().is_empty(), "inputs should be added");
    assert_eq!(tb.outputs().len(), 1, "output should be added");

    let tx = tb.build(wallet.provider()).await?;

    let total_amount_inputs: u64 = tx.inputs().iter().map(|i| i.amount().unwrap()).sum();
    assert!(
        total_amount_inputs > tx.max_fee().unwrap(),
        "amount should cover tx"
    );

    let expected_outputs = vec![Output::change(wallet.address(), 0, AssetId::zeroed())];

    assert_eq!(tx.outputs(), &expected_outputs);

    Ok(())
}

#[tokio::test]
async fn adjust_for_fee_with_message_data_input() -> Result<()> {
    let wallet_signer = PrivateKeySigner::random(&mut rand::thread_rng());
    let receiver_signer = PrivateKeySigner::random(&mut rand::thread_rng());

    let messages = setup_single_message(
        Address::default(),
        wallet_signer.address(),
        100,
        0.into(),
        vec![1, 2, 3], // has data
    );
    let asset_id = AssetId::zeroed();
    let coins = setup_single_asset_coins(wallet_signer.address(), asset_id, 1, 50);
    let provider = setup_test_provider(coins, vec![messages], None, None).await?;
    let wallet = Wallet::new(wallet_signer, provider.clone());
    let receiver = Wallet::new(receiver_signer, provider.clone());

    let amount_to_send = 14;
    let message = wallet.get_messages().await?.pop().unwrap();
    let input = Input::resource_signed(CoinType::Message(message));
    let outputs = wallet.get_asset_outputs_for_amount(receiver.address(), asset_id, amount_to_send);

    {
        // message with data as only input - without adjust for fee
        let mut tb = ScriptTransactionBuilder::prepare_transfer(
            vec![input.clone()],
            outputs.clone(),
            TxPolicies::default(),
        );
        wallet.add_witnesses(&mut tb)?;

        let tx = tb.build(wallet.provider()).await?;
        let err = provider
            .send_transaction_and_await_commit(tx)
            .await
            .unwrap_err();

        assert!(err.to_string().contains("Validity(NoSpendableInput)"));
    }
    {
        // message with data as only input - with adjust for fee
        let mut tb = ScriptTransactionBuilder::prepare_transfer(
            vec![input.clone()],
            outputs.clone(),
            TxPolicies::default(),
        );

        wallet.adjust_for_fee(&mut tb, 0).await.unwrap();
        wallet.add_witnesses(&mut tb)?;

        let tx = tb.build(wallet.provider()).await?;

        assert_eq!(receiver.get_asset_balance(&asset_id).await?, 0);

        provider
            .send_transaction_and_await_commit(tx)
            .await
            .unwrap();

        assert_eq!(receiver.get_asset_balance(&asset_id).await?, amount_to_send);
    }

    Ok(())
}

#[tokio::test]
async fn adjust_fee_resources_to_transfer_with_base_asset() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;

    let base_amount = 30;
    let base_asset_id = AssetId::zeroed();
    let inputs = wallet
        .get_asset_inputs_for_amount(base_asset_id, base_amount.into(), None)
        .await?;
    let outputs =
        wallet.get_asset_outputs_for_amount(Address::zeroed(), base_asset_id, base_amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());

    wallet.adjust_for_fee(&mut tb, base_amount.into()).await?;
    wallet.add_witnesses(&mut tb)?;

    let tx = tb.build(wallet.provider()).await?;

    let total_amount_inputs: u64 = tx.inputs().iter().map(|i| i.amount().unwrap()).sum();
    assert!(total_amount_inputs > tx.max_fee().unwrap()); // can cover tx

    let expected_outputs = vec![
        Output::coin(Address::zeroed(), base_amount, base_asset_id),
        Output::change(wallet.address(), 0, base_asset_id),
    ];

    assert_eq!(tx.outputs(), &expected_outputs);

    Ok(())
}

#[tokio::test]
async fn test_transfer() -> Result<()> {
    let wallet_1_signer = PrivateKeySigner::random(&mut rand::thread_rng());
    let wallet_2_signer = PrivateKeySigner::random(&mut rand::thread_rng());

    let amount = 100;
    let num_coins = 1;
    let base_asset_id = AssetId::zeroed();
    let mut coins_1 =
        setup_single_asset_coins(wallet_1_signer.address(), base_asset_id, num_coins, amount);
    let coins_2 =
        setup_single_asset_coins(wallet_2_signer.address(), base_asset_id, num_coins, amount);
    coins_1.extend(coins_2);

    let provider = setup_test_provider(coins_1, vec![], None, None).await?;
    let wallet_1 = Wallet::new(wallet_1_signer, provider.clone());
    let wallet_2 = Wallet::new(wallet_2_signer, provider.clone()).lock();

    let _ = wallet_1
        .transfer(
            wallet_2.address(),
            amount / 2,
            Default::default(),
            TxPolicies::default(),
        )
        .await
        .unwrap();

    let wallet_2_coins = wallet_2.get_coins(base_asset_id).await.unwrap();
    let wallet_2_balance = wallet_2.get_asset_balance(&base_asset_id).await?;
    assert_eq!(wallet_2_coins.len(), 2);
    assert_eq!(wallet_2_balance, amount + amount / 2);

    Ok(())
}

#[tokio::test]
async fn send_transfer_transactions() -> Result<()> {
    let amount = 5;
    let (wallet_1, wallet_2) = setup_transfer_test(amount).await?;

    // Configure transaction policies
    let tip = 2;
    let script_gas_limit = 500_000;
    let maturity = 0;

    let tx_policies = TxPolicies::default()
        .with_tip(tip)
        .with_maturity(maturity)
        .with_script_gas_limit(script_gas_limit);

    // Transfer 1 from wallet 1 to wallet 2.
    let amount_to_send = 1;
    let base_asset_id = AssetId::zeroed();
    let tx_id = wallet_1
        .transfer(
            wallet_2.address(),
            amount_to_send,
            base_asset_id,
            tx_policies,
        )
        .await?
        .tx_id;

    // Assert that the transaction was properly configured.
    let res = wallet_1
        .try_provider()?
        .get_transaction_by_id(&tx_id)
        .await?
        .unwrap();

    let script: ScriptTransaction = match res.transaction {
        TransactionType::Script(tx) => tx,
        _ => panic!("Received unexpected tx type!"),
    };
    // Transfer scripts uses set `script_gas_limit` despite not having script code
    assert_eq!(script.gas_limit(), script_gas_limit);
    assert_eq!(script.maturity().unwrap(), maturity);

    let wallet_1_spendable_resources = wallet_1
        .get_spendable_resources(base_asset_id, 1, None)
        .await?;
    let wallet_2_spendable_resources = wallet_2
        .get_spendable_resources(base_asset_id, 1, None)
        .await?;
    let wallet_1_all_coins = wallet_1.get_coins(base_asset_id).await?;
    let wallet_2_all_coins = wallet_2.get_coins(base_asset_id).await?;

    // wallet_1 has now only one spent coin
    assert_eq!(wallet_1_spendable_resources.len(), 1);
    assert_eq!(wallet_1_all_coins.len(), 1);
    // Check that wallet two now has a coin.
    assert_eq!(wallet_2_all_coins.len(), 1);
    assert_eq!(wallet_2_spendable_resources.len(), 1);

    Ok(())
}

#[tokio::test]
async fn transfer_coins_with_change() -> Result<()> {
    const AMOUNT: u64 = 5;
    let (wallet_1, wallet_2) = setup_transfer_test(AMOUNT).await?;

    // Transfer 2 from wallet 1 to wallet 2.
    const SEND_AMOUNT: u64 = 2;
    let fee = wallet_1
        .transfer(
            wallet_2.address(),
            SEND_AMOUNT,
            AssetId::zeroed(),
            TxPolicies::default(),
        )
        .await?
        .tx_status
        .total_fee;

    let base_asset_id = AssetId::zeroed();
    let wallet_1_final_coins = wallet_1
        .get_spendable_resources(base_asset_id, 1, None)
        .await?;

    // Assert that we've sent 2 from wallet 1, resulting in an amount of 3 in wallet 1.
    let resulting_amount = wallet_1_final_coins.first().unwrap();
    assert_eq!(resulting_amount.amount(), AMOUNT - SEND_AMOUNT - fee);

    let wallet_2_final_coins = wallet_2.get_coins(base_asset_id).await?;
    assert_eq!(wallet_2_final_coins.len(), 1);

    let total_amount: u64 = wallet_2_final_coins.iter().map(|c| c.amount).sum();
    assert_eq!(total_amount, SEND_AMOUNT);
    Ok(())
}

#[tokio::test]
async fn test_wallet_get_coins() -> Result<()> {
    const AMOUNT: u64 = 1000;
    const NUM_COINS: u64 = 3;
    let addr = Address::zeroed();
    let coins = setup_single_asset_coins(addr, AssetId::zeroed(), NUM_COINS, AMOUNT);

    let provider = setup_test_provider(coins, vec![], None, None).await?;
    let wallet = Wallet::new_locked(addr, provider.clone());

    let consensus_parameters = provider.consensus_parameters().await?;
    let wallet_initial_coins = wallet
        .get_coins(*consensus_parameters.base_asset_id())
        .await?;
    let total_amount: u64 = wallet_initial_coins.iter().map(|c| c.amount).sum();

    assert_eq!(wallet_initial_coins.len(), NUM_COINS as usize);
    assert_eq!(total_amount, AMOUNT * NUM_COINS);

    Ok(())
}

async fn setup_transfer_test(amount: u64) -> Result<(Wallet, Wallet)> {
    let wallet_1_signer = PrivateKeySigner::random(&mut rand::thread_rng());

    let coins = setup_single_asset_coins(wallet_1_signer.address(), AssetId::zeroed(), 1, amount);

    let provider = setup_test_provider(coins, vec![], None, None).await?;

    let wallet_1 = Wallet::new(wallet_1_signer, provider.clone());
    let wallet_2 = Wallet::random(&mut thread_rng(), provider.clone());

    Ok((wallet_1, wallet_2))
}

#[tokio::test]
async fn transfer_more_than_owned() -> Result<()> {
    const AMOUNT: u64 = 1000000;
    let (wallet_1, wallet_2) = setup_transfer_test(AMOUNT).await?;

    // Transferring more than balance should fail.
    let response = wallet_1
        .transfer(
            wallet_2.address(),
            AMOUNT * 2,
            Default::default(),
            TxPolicies::default(),
        )
        .await;

    assert!(response.is_err());

    let wallet_2_coins = wallet_2.get_coins(AssetId::zeroed()).await?;
    assert_eq!(wallet_2_coins.len(), 0);

    Ok(())
}

#[tokio::test]
async fn transfer_coins_of_non_base_asset() -> Result<()> {
    const AMOUNT: u64 = 10000;
    let wallet_1_signer = PrivateKeySigner::random(&mut rand::thread_rng());

    let asset_id: AssetId = AssetId::from([1; 32usize]);
    let mut coins = setup_single_asset_coins(wallet_1_signer.address(), asset_id, 1, AMOUNT);
    // setup base asset coins to pay tx fees
    let base_coins =
        setup_single_asset_coins(wallet_1_signer.address(), AssetId::zeroed(), 1, AMOUNT);
    coins.extend(base_coins);

    let provider = setup_test_provider(coins, vec![], None, None).await?;

    let wallet_1 = Wallet::new(wallet_1_signer, provider.clone());
    let wallet_2 = Wallet::random(&mut thread_rng(), provider.clone());

    const SEND_AMOUNT: u64 = 200;
    let _ = wallet_1
        .transfer(
            wallet_2.address(),
            SEND_AMOUNT,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    let wallet_1_balance = wallet_1.get_asset_balance(&asset_id).await?;
    assert_eq!(wallet_1_balance, AMOUNT - SEND_AMOUNT);

    let wallet_2_final_coins = wallet_2.get_coins(asset_id).await?;
    assert_eq!(wallet_2_final_coins.len(), 1);

    let total_amount: u64 = wallet_2_final_coins.iter().map(|c| c.amount).sum();
    assert_eq!(total_amount, SEND_AMOUNT);

    Ok(())
}

#[tokio::test]
async fn test_transfer_with_multiple_signatures() -> Result<()> {
    let wallet_config = base_asset_wallet_config(5);
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;
    let provider = wallets[0].try_provider()?;

    let receiver = Wallet::random(&mut thread_rng(), provider.clone());

    let amount_to_transfer = 20u64;

    let mut inputs = vec![];
    let consensus_parameters = provider.consensus_parameters().await?;
    for wallet in &wallets {
        inputs.extend(
            wallet
                .get_asset_inputs_for_amount(
                    *consensus_parameters.base_asset_id(),
                    amount_to_transfer.into(),
                    None,
                )
                .await?,
        );
    }

    let amount_to_receive = amount_to_transfer * wallets.len() as u64;

    // all change goes to the first wallet
    let outputs = wallets[0].get_asset_outputs_for_amount(
        receiver.address(),
        *consensus_parameters.base_asset_id(),
        amount_to_receive,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());

    for wallet in wallets.iter() {
        wallet.add_witnesses(&mut tb)?
    }

    let tx = tb.build(provider).await?;
    provider.send_transaction_and_await_commit(tx).await?;

    assert_eq!(
        receiver
            .get_asset_balance(consensus_parameters.base_asset_id())
            .await?,
        amount_to_receive,
    );

    Ok(())
}

#[tokio::test]
async fn wallet_transfer_respects_maturity_and_expiration() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;
    let asset_id = AssetId::zeroed();
    let wallet_balance = wallet.get_asset_balance(&asset_id).await?;

    let provider = wallet.provider();
    let receiver: Address = thread_rng().r#gen();

    let maturity = 10;
    let expiration = 20;
    let tx_policies = TxPolicies::default()
        .with_maturity(maturity)
        .with_expiration(expiration);
    let amount_to_send = 10;

    {
        let err = wallet
            .transfer(receiver, amount_to_send, asset_id, tx_policies)
            .await
            .expect_err("maturity not reached");

        assert!(err.to_string().contains("TransactionMaturity"));
    }
    let transaction_fee = {
        provider.produce_blocks(15, None).await?;
        wallet
            .transfer(receiver, amount_to_send, asset_id, tx_policies)
            .await
            .expect("should succeed. Block height between `maturity` and `expiration`")
            .tx_status
            .total_fee
    };
    {
        provider.produce_blocks(15, None).await?;
        let err = wallet
            .transfer(receiver, amount_to_send, asset_id, tx_policies)
            .await
            .expect_err("expiration reached");

        assert!(err.to_string().contains("TransactionExpiration"));
    }

    // Wallet has spent the funds
    assert_address_balance(
        &wallet.address(),
        provider,
        &asset_id,
        wallet_balance - amount_to_send - transaction_fee,
    )
    .await;

    // Funds were transferred
    assert_address_balance(&receiver, provider, &asset_id, amount_to_send).await;

    Ok(())
}
