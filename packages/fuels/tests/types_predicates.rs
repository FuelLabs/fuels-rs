use fuels::{
    prelude::*,
    tx::AssetId,
    types::{coin::Coin, message::Message},
};
use fuels_accounts::{predicate::Predicate, Account};

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
async fn spend_predicate_coins_messages_single_u64() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicateEncoder",
        abi = "packages/fuels/tests/types/predicates/u64/out/debug/u64-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(32768);
    let code_path = "tests/types/predicates/u64/out/debug/u64.bin";
    let mut predicate: Predicate = Predicate::load_from(code_path)?.with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
    abigen!(Predicate(
        name = "MyPredicateEncoder",
        abi = "packages/fuels/tests/types/predicates/address/out/debug/address-abi.json"
    ));

    let addr: Address = "0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a"
        .parse()
        .unwrap();

    let predicate_data = MyPredicateEncoder::encode_data(addr);

    let mut predicate: Predicate =
        Predicate::load_from("tests/types/predicates/address/out/debug/address.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 100_000;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
        name = "MyPredicateEncoder",
        abi = "packages/fuels/tests/types/predicates/enums/out/debug/enums-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(TestEnum::A(32), AnotherTestEnum::B(32));

    let mut predicate: Predicate =
        Predicate::load_from("tests/types/predicates/enums/out/debug/enums.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
    abigen!(Predicate(
        name = "MyPredicateEncoder",
        abi = "packages/fuels/tests/types/predicates/structs/out/debug/structs-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(
        TestStruct { value: 192 },
        AnotherTestStruct {
            value: 64,
            number: 128,
        },
    );

    let mut predicate: Predicate =
        Predicate::load_from("tests/types/predicates/structs/out/debug/structs.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
        name = "MyPredicateEncoder",
        abi = "packages/fuels/tests/types/predicates/predicate_tuples/out/debug/predicate_tuples-abi.json"
    ));

    let predicate_data =
        MyPredicateEncoder::encode_data((16, TestStruct { value: 32 }, TestEnum::Value(64)), 128);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_tuples/out/debug/predicate_tuples.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
        name = "MyPredicateEncoder",
        abi =
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
    abigen!(Predicate(name="MyPredicateEncoder", abi="packages/fuels/tests/types/predicates/predicate_vectors/out/debug/predicate_vectors-abi.json"));

    let u32_vec = vec![0, 4, 3];
    let vec_in_vec = vec![vec![0, 2, 2], vec![0, 1, 2]];
    let struct_in_vec = vec![SomeStruct { a: 8 }, SomeStruct { a: 1 }];
    let vec_in_struct = SomeStruct { a: vec![0, 16, 2] };
    let array_in_vec = vec![[0u64, 1u64], [32u64, 1u64]];
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

    let vec_in_array = [vec![0, 64, 2], vec![0, 1, 2]];

    let predicate_data = MyPredicateEncoder::encode_data(
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
    );

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vectors/out/debug/predicate_vectors.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
    abigen!(Predicate(name="MyPredicateEncoder", abi="packages/fuels/tests/types/predicates/predicate_generics/out/debug/predicate_generics-abi.json"));

    let generic_struct = GenericStruct { value: 64u8 };
    let generic_struct2 = GenericStruct { value: 64u16 };
    let generic_enum = GenericEnum::Generic(generic_struct2);

    let predicate_data = MyPredicateEncoder::encode_data(generic_struct, generic_enum);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_generics/out/debug/predicate_generics.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(
            receiver.address(),
            predicate_balance,
            asset_id,
            TxParameters::default(),
        )
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
