use fuels::{
    prelude::*,
    tx::AssetId,
    types::{coin::Coin, message::Message},
};

async fn assert_address_balance(
    address: &Bech32Address,
    provider: &Provider,
    asset_id: AssetId,
    amount: u64,
) {
    let balance = provider
        .get_asset_balance(address, asset_id)
        .await
        .expect("Could not retrieve balance");
    assert_eq!(balance, amount);
}

fn get_test_coins_and_messages(
    address: &Bech32Address,
    num_coins: u64,
    num_messages: u64,
    amount: u64,
) -> (Vec<Coin>, Vec<Message>, AssetId) {
    let asset_id = AssetId::default();
    let coins = setup_single_asset_coins(address, asset_id, num_coins, amount);
    let messages = (0..num_messages)
        .map(|i| {
            setup_single_message(
                &Bech32Address::default(),
                address,
                amount,
                i,
                [104, 97, 108, 51, 101].to_vec(),
            )
        })
        .collect();

    (coins, messages, asset_id)
}

// Setup function used to assign coins and messages to a predicate address
// and create a `receiver` wallet
async fn setup_predicate_test(
    predicate_address: &Bech32Address,
    num_coins: u64,
    num_messages: u64,
    amount: u64,
) -> Result<(Provider, u64, WalletUnlocked, u64, AssetId)> {
    let receiver_num_coins = 1;
    let receiver_amount = 1;
    let receiver_balance = receiver_num_coins * receiver_amount;

    let predicate_balance = (num_coins + num_messages) * amount;
    let mut receiver = WalletUnlocked::new_random(None);

    let (mut coins, messages, asset_id) =
        get_test_coins_and_messages(predicate_address, num_coins, num_messages, amount);

    coins.extend(setup_single_asset_coins(
        receiver.address(),
        asset_id,
        receiver_num_coins,
        receiver_amount,
    ));

    let (provider, _address) = setup_test_provider(coins, messages, None, None).await;
    receiver.set_provider(provider.clone());

    Ok((
        provider,
        predicate_balance,
        receiver,
        receiver_balance,
        asset_id,
    ))
}

#[tokio::test]
async fn transfer_coins_and_messages_to_predicate() -> Result<()> {
    let num_coins = 16;
    let num_messages = 32;
    let amount = 64;
    let total_balance = (num_coins + num_messages) * amount;

    let mut wallet = WalletUnlocked::new_random(None);

    let (coins, messages, asset_id) =
        get_test_coins_and_messages(wallet.address(), num_coins, num_messages, amount);

    let (provider, _address) = setup_test_provider(coins, messages, None, None).await;

    wallet.set_provider(provider.clone());

    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/basic_predicate/out/debug/basic_predicate-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/basic_predicate/out/debug/basic_predicate.bin")?;

    predicate
        .receive(&wallet, total_balance, asset_id, None)
        .await?;

    // The predicate has received the funds
    assert_address_balance(predicate.address(), &provider, asset_id, total_balance).await;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_basic() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/basic_predicate/out/debug/basic_predicate-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/basic_predicate/out/debug/basic_predicate.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data(4096, 4097)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(4096, 4096)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await?;

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance,
    )
    .await;

    Ok(())
}
