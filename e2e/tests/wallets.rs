use std::iter::repeat;

use fuel_tx::{input::coin::CoinSigned, Input};
use fuels::{
    prelude::*,
    tx::{TxPointer, UtxoId},
    types::{output::Output, Bytes32},
};

#[tokio::test]
async fn test_wallet_balance_api_multi_asset() -> Result<()> {
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

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    wallet.set_provider(provider);
    let balances = wallet.get_balances().await?;
    assert_eq!(balances.len() as u64, number_of_assets);

    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);

        let expected_key = asset_id.to_string();
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            coins_per_asset * amount_per_coin
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api_single_asset() -> Result<()> {
    let mut wallet = WalletUnlocked::new_random(None);
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        wallet.address(),
        AssetId::zeroed(),
        number_of_coins,
        amount_per_coin,
    );

    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    wallet.set_provider(provider);

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
        number_of_coins * amount_per_coin
    );

    Ok(())
}

fn compare_inputs(inputs: &[Input], expected_inputs: &mut Vec<Input>) -> bool {
    let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);

    // change UTXO_ids to 0s for comparison, because we can't guess the genesis coin ids
    let inputs: Vec<Input> = inputs
        .iter()
        .map(|input| match input {
            Input::CoinSigned(CoinSigned {
                owner,
                amount,
                asset_id,
                tx_pointer,
                witness_index,
                ..
            }) => Input::coin_signed(
                zero_utxo_id,
                *owner,
                *amount,
                *asset_id,
                *tx_pointer,
                *witness_index,
            ),
            other => other.clone(),
        })
        .collect();

    let comparison_results: Vec<bool> = inputs
        .iter()
        .map(|input| {
            let found_index = expected_inputs
                .iter()
                .position(|expected| expected == input);
            if let Some(index) = found_index {
                expected_inputs.remove(index);
                true
            } else {
                false
            }
        })
        .collect();

    if !expected_inputs.is_empty() {
        return false;
    }

    return comparison_results.iter().all(|&r| r);
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
    let wallet_config = base_asset_wallet_config(1);
    let wallet = launch_custom_provider_and_get_wallets(wallet_config, None, None)
        .await?
        .pop()
        .unwrap();

    let mut tb = ScriptTransactionBuilder::prepare_transfer(vec![], vec![], TxPolicies::default());
    tb.add_signer(wallet.clone())?;

    wallet.adjust_for_fee(&mut tb, 0).await?;

    let tx = tb.build(wallet.try_provider()?).await?;

    let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);
    let mut expected_inputs = vec![Input::coin_signed(
        zero_utxo_id,
        wallet.address().into(),
        20,
        AssetId::zeroed(),
        TxPointer::default(),
        0,
    )];
    let expected_outputs = vec![Output::change(
        wallet.address().into(),
        0,
        AssetId::zeroed(),
    )];

    assert!(compare_inputs(tx.inputs(), &mut expected_inputs));
    assert_eq!(tx.outputs(), &expected_outputs);

    Ok(())
}

#[tokio::test]
async fn adjust_fee_resources_to_transfer_with_base_asset() -> Result<()> {
    let wallet_config = base_asset_wallet_config(1);
    let wallet = launch_custom_provider_and_get_wallets(wallet_config, None, None)
        .await?
        .pop()
        .unwrap();

    let base_amount = 30;
    let base_asset_id = AssetId::zeroed();
    let inputs = wallet
        .get_asset_inputs_for_amount(base_asset_id, base_amount)
        .await?;
    let outputs =
        wallet.get_asset_outputs_for_amount(&Address::zeroed().into(), base_asset_id, base_amount);

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    tb.add_signer(wallet.clone())?;

    wallet.adjust_for_fee(&mut tb, base_amount).await?;

    let tx = tb.build(wallet.try_provider()?).await?;

    let zero_utxo_id = UtxoId::new(Bytes32::zeroed(), 0);
    let mut expected_inputs = repeat(Input::coin_signed(
        zero_utxo_id,
        wallet.address().into(),
        20,
        base_asset_id,
        TxPointer::default(),
        0,
    ))
    .take(3)
    .collect::<Vec<_>>();
    let expected_outputs = vec![
        Output::coin(Address::zeroed(), base_amount, base_asset_id),
        Output::change(wallet.address().into(), 0, base_asset_id),
    ];

    assert!(compare_inputs(tx.inputs(), &mut expected_inputs));
    assert_eq!(tx.outputs(), &expected_outputs);

    Ok(())
}

#[tokio::test]
async fn test_transfer() -> Result<()> {
    // Create the actual wallets/signers
    let mut wallet_1 = WalletUnlocked::new_random(None);
    let mut wallet_2 = WalletUnlocked::new_random(None).lock();

    // Setup a coin for each wallet
    let base_asset_id = AssetId::zeroed();
    let mut coins_1 = setup_single_asset_coins(wallet_1.address(), base_asset_id, 1, 1);
    let coins_2 = setup_single_asset_coins(wallet_2.address(), base_asset_id, 1, 1);
    coins_1.extend(coins_2);

    // Setup a provider and node with both set of coins
    let provider = setup_test_provider(coins_1, vec![], None, None).await?;

    // Set provider for wallets
    wallet_1.set_provider(provider.clone());
    wallet_2.set_provider(provider);

    // Transfer 1 from wallet 1 to wallet 2
    let _receipts = wallet_1
        .transfer(
            wallet_2.address(),
            1,
            Default::default(),
            TxPolicies::default(),
        )
        .await
        .unwrap();

    let wallet_2_final_coins = wallet_2.get_coins(base_asset_id).await.unwrap();

    // Check that wallet two now has two coins
    assert_eq!(wallet_2_final_coins.len(), 2);
    Ok(())
}

#[tokio::test]
async fn send_transfer_transactions() -> Result<()> {
    const AMOUNT: u64 = 5;
    let (wallet_1, wallet_2) = setup_transfer_test(AMOUNT).await?;

    // Configure transaction policies
    let tip = 2;
    let script_gas_limit = 500_000;
    let expected_script_gas_limit = 0;
    let maturity = 0;

    let tx_policies = TxPolicies::default()
        .with_tip(tip)
        .with_maturity(maturity)
        .with_script_gas_limit(script_gas_limit);

    // Transfer 1 from wallet 1 to wallet 2.
    const SEND_AMOUNT: u64 = 1;
    let base_asset_id = AssetId::zeroed();
    let (tx_id, _receipts) = wallet_1
        .transfer(wallet_2.address(), SEND_AMOUNT, base_asset_id, tx_policies)
        .await?;

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
    // Transfer scripts have `script_gas_limit` set to `0`
    assert_eq!(script.gas_limit(), expected_script_gas_limit);
    assert_eq!(script.maturity(), maturity as u32);

    let wallet_1_spendable_resources = wallet_1.get_spendable_resources(base_asset_id, 1).await?;
    let wallet_2_spendable_resources = wallet_2.get_spendable_resources(base_asset_id, 1).await?;
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
    let _receipts = wallet_1
        .transfer(
            wallet_2.address(),
            SEND_AMOUNT,
            AssetId::zeroed(),
            TxPolicies::default(),
        )
        .await?;

    let base_asset_id = AssetId::zeroed();
    let wallet_1_final_coins = wallet_1.get_spendable_resources(base_asset_id, 1).await?;

    // Assert that we've sent 2 from wallet 1, resulting in an amount of 3 in wallet 1.
    let resulting_amount = wallet_1_final_coins.first().unwrap();
    assert_eq!(resulting_amount.amount(), AMOUNT - SEND_AMOUNT);

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
    let mut wallet = WalletUnlocked::new_random(None);
    let coins = setup_single_asset_coins(wallet.address(), AssetId::zeroed(), NUM_COINS, AMOUNT);

    let provider = setup_test_provider(coins, vec![], None, None).await?;
    wallet.set_provider(provider.clone());

    let wallet_initial_coins = wallet.get_coins(*provider.base_asset_id()).await?;
    let total_amount: u64 = wallet_initial_coins.iter().map(|c| c.amount).sum();

    assert_eq!(wallet_initial_coins.len(), NUM_COINS as usize);
    assert_eq!(total_amount, AMOUNT * NUM_COINS);

    Ok(())
}

async fn setup_transfer_test(amount: u64) -> Result<(WalletUnlocked, WalletUnlocked)> {
    let mut wallet_1 = WalletUnlocked::new_random(None);
    let mut wallet_2 = WalletUnlocked::new_random(None);

    let coins = setup_single_asset_coins(wallet_1.address(), AssetId::zeroed(), 1, amount);

    let provider = setup_test_provider(coins, vec![], None, None).await?;

    wallet_1.set_provider(provider.clone());
    wallet_2.set_provider(provider);

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
    let mut wallet_1 = WalletUnlocked::new_random(None);
    let mut wallet_2 = WalletUnlocked::new_random(None);

    let asset_id: AssetId = AssetId::from([1; 32usize]);
    let mut coins = setup_single_asset_coins(wallet_1.address(), asset_id, 1, AMOUNT);
    // setup base asset coins to pay tx fees
    let base_coins = setup_single_asset_coins(wallet_1.address(), AssetId::zeroed(), 1, AMOUNT);
    coins.extend(base_coins);

    let provider = setup_test_provider(coins, vec![], None, None).await?;

    wallet_1.set_provider(provider.clone());
    wallet_2.set_provider(provider);

    const SEND_AMOUNT: u64 = 200;
    let _receipts = wallet_1
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

    let mut receiver = WalletUnlocked::new_random(None);
    receiver.set_provider(provider.clone());

    let amount_to_transfer = 20;

    let mut inputs = vec![];
    for wallet in &wallets {
        inputs.extend(
            wallet
                .get_asset_inputs_for_amount(*provider.base_asset_id(), amount_to_transfer)
                .await?,
        );
    }

    let amount_to_receive = amount_to_transfer * wallets.len() as u64;

    // all change goes to the first wallet
    let outputs = wallets[0].get_asset_outputs_for_amount(
        receiver.address(),
        *provider.base_asset_id(),
        amount_to_receive,
    );

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());

    for wallet in wallets.iter() {
        tb.add_signer(wallet.clone())?;
    }

    let tx = tb.build(provider).await?;
    provider.send_transaction_and_await_commit(tx).await?;

    assert_eq!(
        receiver.get_asset_balance(provider.base_asset_id()).await?,
        amount_to_receive,
    );

    Ok(())
}
