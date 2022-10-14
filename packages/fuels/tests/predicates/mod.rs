use fuels::core::abi_encoder::ABIEncoder;
use fuels::prelude::*;
use std::str::FromStr;

async fn setup_predicate_test(
    file_path: &str,
    num_coins: u64,
    coin_amount: u64,
) -> Result<(Predicate, WalletUnlocked, WalletUnlocked, AssetId), Error> {
    let predicate = Predicate::load_from(file_path)?;

    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(2), Some(num_coins), Some(coin_amount)),
        Some(Config {
            predicates: true,
            utxo_validation: true,
            ..Config::local_node()
        }),
    )
    .await;

    let sender = wallets.pop().unwrap();
    let receiver = wallets.pop().unwrap();
    let asset_id = AssetId::default();

    Ok((predicate, sender, receiver, asset_id))
}

#[tokio::test]
async fn can_call_no_arg_predicate_returns_true() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_true/out/debug/predicate_true.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 2;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_no_arg_predicate_returns_false() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_false/out/debug/predicate_false.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 4;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_u32_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_u32/out/debug/predicate_u32.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 8;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    // invalid predicate data
    let predicate_data = ABIEncoder::encode(&[101_u32.into_token()])
        .unwrap()
        .resolve(0);
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);

    // valid predicate data
    let predicate_data = ABIEncoder::encode(&[1078_u32.into_token()])
        .unwrap()
        .resolve(0);
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_address_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_address/out/debug/predicate_address.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 16;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    let addr =
        Address::from_str("0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a")
            .unwrap();
    let predicate_data = ABIEncoder::encode(&[addr.into_token()]).unwrap().resolve(0);
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn can_call_predicate_with_struct_data() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_struct/out/debug/predicate_struct.bin",
        1,
        16,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 8;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 16);

    // invalid predicate data
    let predicate_data = ABIEncoder::encode(&[true.into_token(), 55_u32.into_token()])
        .unwrap()
        .resolve(0);
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await
        .expect_err("should error");

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, receiver_balance_after);

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, amount_to_predicate);

    // valid predicate data
    let predicate_data = ABIEncoder::encode(&[true.into_token(), 100_u32.into_token()])
        .unwrap()
        .resolve(0);
    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            Some(predicate_data),
            TxParameters::default(),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 0);
    Ok(())
}

#[tokio::test]
async fn predicate_with_multiple_coins() -> Result<(), Error> {
    let (predicate, sender, receiver, asset_id) = setup_predicate_test(
        "tests/predicates/predicate_true/out/debug/predicate_true.bin",
        3,
        100,
    )
    .await?;
    let provider = receiver.get_provider()?;
    let amount_to_predicate = 10;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    sender
        .transfer(
            predicate.address(),
            amount_to_predicate,
            asset_id,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    let receiver_balance_before = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(receiver_balance_before, 300);

    receiver
        .receive_from_predicate(
            predicate.address(),
            predicate.code(),
            amount_to_predicate,
            asset_id,
            None,
            TxParameters::new(Some(1), None, None),
        )
        .await?;

    let receiver_balance_after = provider
        .get_asset_balance(receiver.address(), asset_id)
        .await?;
    assert_eq!(
        receiver_balance_before + amount_to_predicate - 1,
        receiver_balance_after
    );

    let predicate_balance = provider
        .get_asset_balance(predicate.address(), asset_id)
        .await?;
    assert_eq!(predicate_balance, 10);
    Ok(())
}
