use std::path::Path;

use fuels::{
    accounts::{predicate::Predicate, Account},
    prelude::*,
    types::{coin::Coin, message::Message, unresolved_bytes::UnresolvedBytes, AssetId},
};

async fn assert_predicate_spendable(
    data: UnresolvedBytes,
    project_path: impl AsRef<Path>,
) -> Result<()> {
    let binary_path = project_binary(project_path);
    let mut predicate: Predicate = Predicate::load_from(&binary_path)?.with_data(data);

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

fn project_binary(project_root: impl AsRef<Path>) -> String {
    let project_root = project_root.as_ref();

    let project_name = project_root
        .file_name()
        .expect("Couldn't extract project name")
        .to_str()
        .unwrap();

    project_root
        .join(format!("out/debug/{project_name}.bin"))
        .display()
        .to_string()
}

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
        .map(|i| setup_single_message(&Bech32Address::default(), address, amount, i.into(), vec![]))
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
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/u64/out/debug/u64-abi.json"
    ));

    let data = MyPredicateEncoder::encode_data(32768);

    assert_predicate_spendable(data, "tests/types/predicates/u64").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_address() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/address/out/debug/address-abi.json"
    ));

    let addr: Address = "0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a"
        .parse()
        .unwrap();

    let data = MyPredicateEncoder::encode_data(addr);

    assert_predicate_spendable(data, "tests/types/predicates/address").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_enums() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/enums/out/debug/enums-abi.json"
    ));

    let data = MyPredicateEncoder::encode_data(TestEnum::A(32), AnotherTestEnum::B(32));

    assert_predicate_spendable(data, "tests/types/predicates/enums").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_structs() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/structs/out/debug/structs-abi.json"
    ));

    let data = MyPredicateEncoder::encode_data(
        TestStruct { value: 192 },
        AnotherTestStruct {
            value: 64,
            number: 128,
        },
    );

    assert_predicate_spendable(data, "tests/types/predicates/structs").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_tuple() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/predicate_tuples/out/debug/predicate_tuples-abi.json"
    ));

    let data =
        MyPredicateEncoder::encode_data((16, TestStruct { value: 32 }, TestEnum::Value(64)), 128);

    assert_predicate_spendable(data, "tests/types/predicates/predicate_tuples").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_vector() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let data = MyPredicateEncoder::encode_data(18, 24, vec![2, 4, 42]);

    assert_predicate_spendable(data, "tests/types/predicates/predicate_vector").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_vectors() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/types/predicates/predicate_vectors/out/debug/predicate_vectors-abi.json"));

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

    let data = MyPredicateEncoder::encode_data(
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

    assert_predicate_spendable(data, "tests/types/predicates/predicate_vectors").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_generics() -> Result<()> {
    abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/types/predicates/predicate_generics/out/debug/predicate_generics-abi.json"));

    let data = MyPredicateEncoder::encode_data(
        GenericStruct { value: 64u8 },
        GenericEnum::Generic(GenericStruct { value: 64u16 }),
    );

    assert_predicate_spendable(data, "tests/types/predicates/predicate_generics").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_bytes() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/predicate_bytes/out/debug/predicate_bytes-abi.json"
    ));

    let bytes = Bytes(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![bytes.clone(), bytes.clone()],
        inner_enum: SomeEnum::Second(bytes),
    };

    let data = MyPredicateEncoder::encode_data(wrapper);

    assert_predicate_spendable(data, "tests/types/predicates/predicate_bytes").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_raw_slice() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/predicate_raw_slice/out/debug/predicate_raw_slice-abi.json"
    ));

    let raw_slice = RawSlice(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![raw_slice.clone(), raw_slice.clone()],
        inner_enum: SomeEnum::Second(raw_slice),
    };

    let data = MyPredicateEncoder::encode_data(wrapper);

    assert_predicate_spendable(data, "tests/types/predicates/predicate_raw_slice").await?;

    Ok(())
}

fn u128_from_parts(upper: u64, lower: u64) -> u128 {
    let bytes: [u8; 16] = [upper.to_be_bytes(), lower.to_be_bytes()]
        .concat()
        .try_into()
        .unwrap();
    u128::from_be_bytes(bytes)
}

#[tokio::test]
async fn predicate_handles_u128() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/types/predicates/predicate_u128/out/debug/predicate_u128-abi.json"
    ));

    let data = MyPredicateEncoder::encode_data(u128_from_parts(8, 2));
    assert_predicate_spendable(data, "tests/types/predicates/predicate_u128").await?;

    Ok(())
}
