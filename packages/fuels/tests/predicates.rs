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
        .flat_map(|i| {
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
) -> Result<(Provider, u64, WalletUnlocked, u64, AssetId), Error> {
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
async fn transfer_coins_and_messages_to_predicate() -> Result<(), Error> {
    let num_coins = 16;
    let num_messages = 32;
    let amount = 64;
    let total_balance = (num_coins + num_messages) * amount;

    let mut wallet = WalletUnlocked::new_random(None);

    let (coins, messages, asset_id) =
        get_test_coins_and_messages(wallet.address(), num_coins, num_messages, amount);

    let (provider, _address) = setup_test_provider(coins, messages, None, None).await;

    wallet.set_provider(provider.clone());

    predicate_abigen!(
        MyPredicate,
        "packages/fuels/tests/predicates/predicate_struct/out/debug/predicate_struct-abi.json"
    );

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_struct/out/debug/predicate_struct.bin")?;

    predicate
        .receive_from_wallet(&wallet, total_balance, asset_id, None)
        .await?;

    // The predicate has received the funds
    assert_address_balance(predicate.address(), &provider, asset_id, total_balance).await;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_vector_args() -> Result<(), Error> {
    predicate_abigen!(
        MyPredicate,
        "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    );

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data(2, 4, vec![2, 4, 43])
        .spend_to_wallet(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(2, 4, vec![2, 4, 42])
        .spend_to_wallet(&receiver, predicate_balance, asset_id, None)
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

// #[tokio::test]
// async fn spend_predicate_with_vector_args() -> Result<(), Error> {
//     let num_coins = 2;
//     let amount = 4;
//     let (sender, receiver, provider, asset_id) =
//         setup_predicate_coin_test(num_coins, amount).await?;
//     let initial_balance = num_coins * amount;

//     let transfer_amount = initial_balance / 2;

//     predicate_abigen!(
//         MyPredicate,
//         "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
//     );

//     let predicate =
//         MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?;

//     // Make two transactions so that the predicate holds 2 coins. This is needed to verify
//     // that the predicate data offsets are correctly calculated for multiple coins
//     predicate
//         .receive_from_wallet(&sender, transfer_amount, asset_id, None)
//         .await?;

//     predicate
//         .receive_from_wallet(&sender, transfer_amount, asset_id, None)
//         .await?;

//     // The predicate has received the funds
//     assert_address_balance(predicate.address(), &provider, asset_id, initial_balance).await;

//     // Run predicate with wrong data
//     predicate
//         .encode_data(2, 4, vec![2, 4, 43])
//         .spend_to_wallet(&receiver, initial_balance, asset_id, None)
//         .await
//         .expect_err("Should error");

//     // No funds were transferred
//     assert_address_balance(receiver.address(), &provider, asset_id, initial_balance).await;

//     predicate
//         .encode_data(2, 4, vec![2, 4, 42])
//         .spend_to_wallet(&receiver, initial_balance, asset_id, None)
//         .await?;

//     // The predicate has spent the funds
//     assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

//     // Funds were transferred
//     assert_address_balance(
//         receiver.address(),
//         &provider,
//         asset_id,
//         initial_balance + transfer_amount * 2,
//     )
//     .await;

//     Ok(())
// }

// #[tokio::test]
// async fn can_call_predicate_with_u32_data_new() -> Result<(), Error> {
//     let initial_balance = 16;
//     let (sender, receiver, asset_id) = setup_predicate_test2(1, initial_balance).await?;
//     let provider = receiver.get_provider()?;
//     let amount = 8;

//     predicate_abigen!(
//         MyPredicate,
//         "packages/fuels/tests/predicates/predicate_u32/out/debug/predicate_u32-abi.json"
//     );

//     let predicate =
//         MyPredicate::load_from("tests/predicates/predicate_u32/out/debug/predicate_u32.bin")?;

//     predicate
//         .receive_from_wallet(&sender, amount, asset_id, None)
//         .await?;

//     // The predicate has received the funds
//     assert_address_balance(predicate.address(), provider, asset_id, amount).await;

//     // Run predicate with wrong data
//     predicate
//         .encode_data(1077)
//         .spend_to_wallet(&receiver, amount, asset_id, None)
//         .await
//         .expect_err("Should error");

//     // No funds were transferred
//     assert_address_balance(receiver.address(), provider, asset_id, initial_balance).await;

//     predicate
//         .encode_data(1078)
//         .spend_to_wallet(&receiver, amount, asset_id, None)
//         .await?;

//     // The predicate has spent the funds
//     assert_address_balance(predicate.address(), provider, asset_id, 0).await;

//     // Funds were transferred
//     assert_address_balance(
//         receiver.address(),
//         provider,
//         asset_id,
//         initial_balance + amount,
//     )
//     .await;

//     Ok(())
// }

// #[tokio::test]
// async fn can_call_predicate_with_struct_data_new() -> Result<(), Error> {
//     let initial_balance = 16;
//     let (sender, receiver, asset_id) = setup_predicate_test2(1, initial_balance).await?;
//     let provider = receiver.get_provider()?;
//     let amount = 8;

//     predicate_abigen!(
//         MyPredicate,
//         "packages/fuels/tests/predicates/predicate_struct/out/debug/predicate_struct-abi.json"
//     );

//     let predicate =
//         MyPredicate::load_from("tests/predicates/predicate_struct/out/debug/predicate_struct.bin")?;

//     predicate
//         .receive_from_wallet(&sender, amount, asset_id, None)
//         .await?;

//     // The predicate has received the funds
//     assert_address_balance(predicate.address(), provider, asset_id, amount).await;

//     // Run predicate with wrong data
//     predicate
//         .encode_data(Validation {
//             has_account: false,
//             total_complete: 10,
//         })
//         .spend_to_wallet(&receiver, amount, asset_id, None)
//         .await
//         .expect_err("Should error");

//     // No funds were transferred
//     assert_address_balance(receiver.address(), provider, asset_id, initial_balance).await;

//     predicate
//         .encode_data(Validation {
//             has_account: true,
//             total_complete: 100,
//         })
//         .spend_to_wallet(&receiver, amount, asset_id, None)
//         .await?;

//     // The predicate has spent the funds
//     assert_address_balance(predicate.address(), provider, asset_id, 0).await;

//     // Funds were transferred
//     assert_address_balance(
//         receiver.address(),
//         provider,
//         asset_id,
//         initial_balance + amount,
//     )
//     .await;

//     Ok(())
// }



