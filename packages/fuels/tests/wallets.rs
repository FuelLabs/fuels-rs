use fuels::prelude::*;

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
    let balances = wallet
        .get_balances(number_of_assets * coins_per_asset)
        .await?;
    assert_eq!(balances.len() as u64, number_of_assets);

    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);

        assert!(balances.contains_key(&asset_id));
        assert_eq!(
            *balances.get(&asset_id).unwrap(),
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

    let balances = wallet.get_balances(number_of_coins).await?;
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&BASE_ASSET_ID));
    assert_eq!(
        *balances.get(&BASE_ASSET_ID).unwrap(),
        number_of_coins * amount_per_coin
    );

    Ok(())
}
