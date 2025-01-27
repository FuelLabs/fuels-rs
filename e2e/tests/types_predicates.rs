use std::{default::Default, path::Path};

use fuels::{
    accounts::{predicate::Predicate, Account},
    prelude::*,
    types::{coin::Coin, message::Message, AssetId, Bits256, U256},
};

async fn assert_predicate_spendable(data: Vec<u8>, project_path: impl AsRef<Path>) -> Result<()> {
    let binary_path = project_binary(project_path);
    let mut predicate: Predicate = Predicate::load_from(&binary_path)?.with_data(data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let amount_to_send = 42;
    predicate
        .transfer(receiver.address(), 42, asset_id, TxPolicies::default())
        .await?;

    // The predicate has spent the funds
    //TODO:https://github.com/FuelLabs/fuels-rs/issues/1394
    // assert that the amount_to_send + fee was deducted from the predicate
    // assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + amount_to_send,
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
        .join(format!("out/release/{project_name}.bin"))
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
    let asset_id = AssetId::zeroed();
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

    let node_config = NodeConfig {
        starting_gas_price: 0,
        ..Default::default()
    };
    let provider = setup_test_provider(coins, messages, Some(node_config), None).await?;
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
        abi = "e2e/sway/types/predicates/u64/out/release/u64-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(32768)?;

    assert_predicate_spendable(data, "sway/types/predicates/u64").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_address() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/address/out/release/address-abi.json"
    ));

    let addr: Address =
        "0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a".parse()?;

    let data = MyPredicateEncoder::default().encode_data(addr)?;

    assert_predicate_spendable(data, "sway/types/predicates/address").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_enums() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/enums/out/release/enums-abi.json"
    ));

    let data =
        MyPredicateEncoder::default().encode_data(TestEnum::A(32), AnotherTestEnum::B(32))?;

    assert_predicate_spendable(data, "sway/types/predicates/enums").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_structs() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/structs/out/release/structs-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(
        TestStruct { value: 192 },
        AnotherTestStruct {
            value: 64,
            number: 128,
        },
    )?;

    assert_predicate_spendable(data, "sway/types/predicates/structs").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_tuple() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_tuples/out/release/predicate_tuples-abi.json"
    ));

    let data = MyPredicateEncoder::default()
        .encode_data((16, TestStruct { value: 32 }, TestEnum::Value(64)), 128)?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_tuples").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_vector() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(18, 24, vec![2, 4, 42])?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_vector").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_vectors() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_vectors/out/release/predicate_vectors-abi.json"
    ));

    let u32_vec = vec![0, 4, 3];
    let vec_in_vec = vec![vec![0, 2, 2], vec![0, 1, 2]];
    let struct_in_vec = vec![SomeStruct { a: 8 }, SomeStruct { a: 1 }];
    let vec_in_struct = SomeStruct { a: vec![0, 16, 2] };
    let array_in_vec = vec![[0u64, 1u64], [32u64, 1u64]];
    let vec_in_enum = SomeEnum::A(vec![0, 1, 128]);
    let enum_in_vec = vec![SomeEnum::A(0), SomeEnum::A(16)];
    let b256_in_vec = vec![Bits256([2; 32]), Bits256([2; 32])];
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

    let data = MyPredicateEncoder::default().encode_data(
        u32_vec,
        vec_in_vec,
        struct_in_vec,
        vec_in_struct,
        array_in_vec,
        vec_in_array,
        vec_in_enum,
        enum_in_vec,
        b256_in_vec,
        tuple_in_vec,
        vec_in_tuple,
        vec_in_a_vec_in_a_struct_in_a_vec,
    )?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_vectors").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_generics() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "e2e/sway/types/predicates/predicate_generics/out/release/predicate_generics-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(
        GenericStruct { value: 64u8 },
        GenericEnum::Generic(GenericStruct { value: 64u16 }),
    )?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_generics").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_bytes_hash() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_bytes_hash/out/release/predicate_bytes_hash-abi.json"

    ));

    let bytes = Bytes::from_hex_str(
        "0x75a448b91bb82a255757e61ba3eb7afe282c09842485268d4d72a027ec0cffc80500000000",
    )?;

    let bits256 = Bits256::from_hex_str(
        "0x173d69ea3d0aa050d01ff7cc60ccd4579b567c465cd115c6876c2da4a332fb99",
    )?;

    let data = MyPredicateEncoder::default().encode_data(bytes, bits256)?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_bytes_hash").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_bytes() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_bytes/out/release/predicate_bytes-abi.json"
    ));

    let bytes = Bytes(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![bytes.clone(), bytes.clone()],
        inner_enum: SomeEnum::Second(bytes),
    };

    let data = MyPredicateEncoder::default().encode_data(wrapper)?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_bytes").await?;

    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_raw_slice() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_raw_slice/out/release/predicate_raw_slice-abi.json"
    ));

    let raw_slice = RawSlice(vec![40, 41, 42]);
    let wrapper = Wrapper {
        inner: vec![raw_slice.clone(), raw_slice.clone()],
        inner_enum: SomeEnum::Second(raw_slice),
    };

    let data = MyPredicateEncoder::default().encode_data(wrapper)?;

    assert_predicate_spendable(data, "sway/types/predicates/predicate_raw_slice").await?;

    Ok(())
}

fn u128_from(parts: (u64, u64)) -> u128 {
    let bytes: [u8; 16] = [parts.0.to_be_bytes(), parts.1.to_be_bytes()]
        .concat()
        .try_into()
        .unwrap();
    u128::from_be_bytes(bytes)
}

#[tokio::test]
async fn predicate_handles_u128() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_u128/out/release/predicate_u128-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(u128_from((8, 2)))?;
    assert_predicate_spendable(data, "sway/types/predicates/predicate_u128").await?;

    Ok(())
}

#[tokio::test]
async fn predicate_handles_b256() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_b256/out/release/predicate_b256-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(Bits256([1; 32]))?;
    assert_predicate_spendable(data, "sway/types/predicates/predicate_b256").await?;

    Ok(())
}

fn u256_from(parts: (u64, u64, u64, u64)) -> U256 {
    let bytes: [u8; 32] = [
        parts.0.to_be_bytes(),
        parts.1.to_be_bytes(),
        parts.2.to_be_bytes(),
        parts.3.to_be_bytes(),
    ]
    .concat()
    .try_into()
    .unwrap();
    U256::from(bytes)
}

#[tokio::test]
async fn predicate_handles_u256() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_u256/out/release/predicate_u256-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(u256_from((10, 11, 12, 13)))?;
    assert_predicate_spendable(data, "sway/types/predicates/predicate_u256").await?;

    Ok(())
}

#[tokio::test]
async fn predicate_handles_std_string() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_std_lib_string/out/release/predicate_std_lib_string-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data(10, 11, String::from("Hello World"))?;
    assert_predicate_spendable(data, "sway/types/predicates/predicate_std_lib_string").await?;

    Ok(())
}

#[tokio::test]
async fn predicate_string_slice() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_string_slice/out/release/predicate_string_slice-abi.json"
    ));

    let data = MyPredicateEncoder::default().encode_data("predicate-input".try_into()?)?;
    assert_predicate_spendable(data, "sway/types/predicates/predicate_string_slice").await?;

    Ok(())
}
