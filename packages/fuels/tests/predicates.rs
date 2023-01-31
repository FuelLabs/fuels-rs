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
        abi = "packages/fuels/tests/predicates/predicate_basic/out/debug/predicate_basic-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_basic/out/debug/predicate_basic.bin")?;

    predicate
        .receive(&wallet, total_balance, asset_id, None)
        .await?;

    // The predicate has received the funds
    assert_address_balance(predicate.address(), &provider, asset_id, total_balance).await;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_single_u64() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_u64/out/debug/predicate_u64-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_u64/out/debug/predicate_u64.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data(32767)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(32768)
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

#[tokio::test]
async fn spend_predicate_coins_messages_basic() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_basic/out/debug/predicate_basic-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_basic/out/debug/predicate_basic.bin")?;

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

#[tokio::test]
async fn spend_predicate_coins_messages_address() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_address/out/debug/predicate_address-abi.json"));

    let predicate = MyPredicate::load_from(
        "tests/predicates/predicate_address/out/debug/predicate_address.bin",
    )?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    let wrong_addr: Address = "0x7f86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a"
        .parse()
        .unwrap();

    // Run predicate with wrong data
    predicate
        .encode_data(wrong_addr)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    let addr: Address = "0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a"
        .parse()
        .unwrap();

    predicate
        .encode_data(addr)
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

#[tokio::test]
async fn spend_predicate_coins_messages_enums() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_enums/out/debug/predicate_enums-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_enums/out/debug/predicate_enums.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data(TestEnum::A(32), AnotherTestEnum::A(32))
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(TestEnum::A(32), AnotherTestEnum::B(32))
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

#[tokio::test]
async fn spend_predicate_coins_messages_structs() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_structs/out/debug/predicate_structs-abi.json"));

    let predicate = MyPredicate::load_from(
        "tests/predicates/predicate_structs/out/debug/predicate_structs.bin",
    )?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data(
            TestStruct { value: 191 },
            AnotherTestStruct {
                value: 63,
                number: 127,
            },
        )
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(
            TestStruct { value: 192 },
            AnotherTestStruct {
                value: 64,
                number: 128,
            },
        )
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

#[tokio::test]
async fn spend_predicate_coins_messages_tuple() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_tuple/out/debug/predicate_tuple-abi.json"
    ));

    let predicate =
        MyPredicate::load_from("tests/predicates/predicate_tuple/out/debug/predicate_tuple.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // Run predicate with wrong data
    predicate
        .encode_data((15, TestStruct { value: 31 }, TestEnum::Value(63)), 127)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data((16, TestStruct { value: 32 }, TestEnum::Value(64)), 128)
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

#[tokio::test]
async fn spend_predicate_coins_messages_vector() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

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
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    predicate
        .encode_data(2, 4, vec![2, 4, 42])
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

#[tokio::test]
async fn spend_predicate_coins_messages_vectors() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_vectors/out/debug/predicate_vectors-abi.json"));

    let predicate = MyPredicate::load_from(
        "tests/predicates/predicate_vectors/out/debug/predicate_vectors.bin",
    )?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    let u32_vec = vec![0, 4, 3];
    let vec_in_vec = vec![vec![0, 2, 2], vec![0, 1, 2]];
    let struct_in_vec = vec![SomeStruct { a: 8 }, SomeStruct { a: 1 }];
    let vec_in_struct = SomeStruct { a: vec![0, 16, 2] };
    let array_in_vec = vec![[0u64, 1u64], [32u64, 1u64]];
    let vec_in_array = [vec![0, 63, 3], vec![0, 1, 2]];
    let vec_in_enum = SomeEnum::A(vec![0, 1, 128]);
    let enum_in_vec = vec![SomeEnum::A(0), SomeEnum::A(16)];
    let tuple_in_vec = vec![(0, 0), (128, 1)];
    let vec_in_tuple = (vec![0, 64, 2], vec![0, 1, 2]);
    let vec_in_a_vec_in_a_struct_in_a_vec = vec![
        SomeStruct {
            a: vec![vec![0, 1, 2], vec![3, 4, 5]],
        },
        SomeStruct {
            a: vec![vec![6, 7, 8], vec![9, 32, 11]],
        },
    ];

    // Run predicate with wrong data
    predicate
        .encode_data(
            u32_vec.clone(),
            vec_in_vec.clone(),
            struct_in_vec.clone(),
            vec_in_struct.clone(),
            array_in_vec.clone(),
            vec_in_array,
            vec_in_enum.clone(),
            enum_in_vec.clone(),
            tuple_in_vec.clone(),
            vec_in_tuple.clone(),
            vec_in_a_vec_in_a_struct_in_a_vec.clone(),
        )
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    let vec_in_array = [vec![0, 64, 2], vec![0, 1, 2]];

    predicate
        .encode_data(
            u32_vec,
            vec_in_vec,
            struct_in_vec,
            vec_in_struct,
            array_in_vec,
            vec_in_array,
            vec_in_enum,
            enum_in_vec,
            tuple_in_vec,
            vec_in_tuple,
            vec_in_a_vec_in_a_struct_in_a_vec,
        )
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

#[tokio::test]
async fn spend_predicate_coins_messages_generics() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_generics/out/debug/predicate_generics-abi.json"));

    let predicate = MyPredicate::load_from(
        "tests/predicates/predicate_generics/out/debug/predicate_generics.bin",
    )?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    let generic_struct = GenericStruct { value: 64 };
    let generic_enum = GenericEnum::AnotherGeneric(64);

    // Run predicate with wrong data
    predicate
        .encode_data(generic_struct, generic_enum)
        .spend(&receiver, predicate_balance, asset_id, None)
        .await
        .expect_err("Should error");

    // No funds were transferred
    assert_address_balance(receiver.address(), &provider, asset_id, receiver_balance).await;

    let generic_struct = GenericStruct { value: 64u8 };
    let generic_struct2 = GenericStruct { value: 64u16 };
    let generic_enum = GenericEnum::Generic(generic_struct2);

    predicate
        .encode_data(generic_struct, generic_enum)
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
