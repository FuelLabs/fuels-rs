use fuels::{
    prelude::*,
    tx::AssetId,
    types::{coin::Coin, message::Message},
};
use fuels_signers::predicate::Predicate;
use fuels_signers::Account;

// use crate::my_predicate_mod::MyPredicate;
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

    coins.extend(setup_single_asset_coins(
        predicate_address,
        AssetId::from([1u8; 32]),
        num_coins,
        amount,
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

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_basic/out/debug/predicate_basic.bin")?
            .get_predicate();

    predicate.set_provider(provider.clone());

    wallet
        .transfer(predicate.address(), total_balance, asset_id, None)
        .await?;

    //     The predicate has received the funds
    assert_address_balance(predicate.address(), &provider, asset_id, total_balance).await;
    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_single_u64() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_u64/out/debug/predicate_u64-abi.json"
    ));

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_u64/out/debug/predicate_u64.bin")?
            .set_data(32768)
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_basic/out/debug/predicate_basic.bin")?
            .set_data(4097, 4097)
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let addr: Address = "0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a"
        .parse()
        .unwrap();

    let mut predicate: Predicate = MyPredicate::load_from(
        "tests/predicates/predicate_address/out/debug/predicate_address.bin",
    )?
    .set_data(addr)
    .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 100_000;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_enums/out/debug/predicate_enums.bin")?
            .set_data(TestEnum::A(32), AnotherTestEnum::B(32))
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate = MyPredicate::load_from(
        "tests/predicates/predicate_structs/out/debug/predicate_structs.bin",
    )?
    .set_data(
        TestStruct { value: 192 },
        AnotherTestStruct {
            value: 64,
            number: 128,
        },
    )
    .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_tuple/out/debug/predicate_tuple.bin")?
            .set_data((16, TestStruct { value: 32 }, TestEnum::Value(64)), 128)
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let mut predicate: Predicate = MyPredicate::load_from(
        "tests/predicates/predicate_vectors/out/debug/predicate_vectors.bin",
    )?
    .set_data(
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
    .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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

    let generic_struct = GenericStruct { value: 64u8 };
    let generic_struct2 = GenericStruct { value: 64u16 };
    let generic_enum = GenericEnum::Generic(generic_struct2);

    let mut predicate: Predicate = MyPredicate::load_from(
        "tests/predicates/predicate_generics/out/debug/predicate_generics.bin",
    )?
    .set_data(generic_struct, generic_enum)
    .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    predicate
        .transfer(receiver.address(), predicate_balance, asset_id, None)
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
async fn pay_with_predicate() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi = "packages/fuels/tests/predicates/predicate_u64/out/debug/predicate_u64-abi.json"
        )
    );

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_u64/out/debug/predicate_u64.bin")?
            .set_data(32768)
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), predicate.clone());
    let tx_params = TxParameters::new(Some(1000000), Some(10000), None);

    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        192
    );

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(tx_params)
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);
    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        187
    );

    Ok(())
}

// #[tokio::test]
// async fn test_basic_script_with_tx_parameters_predicate() -> Result<()> {
//     abigen!(
//         Script(
//             name = "bimbam_script",
//             abi = "packages/fuels/tests/scripts/basic_script/out/debug/basic_script-abi.json"
//     ),
//         Predicate(
//             name = "MyPredicate",
//             abi = "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
//         )
//     );
//
//     let mut predicate: Predicate =
//         MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
//             .set_data(2, 4, vec![2, 4, 42])
//             .get_predicate();
//
//     let num_coins = 4;
//     let num_messages = 8;
//     let amount = 16;
//     let (provider, _, _, _, _) =
//         setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;
//
//     predicate.set_provider(provider.clone());
//
//     let bin_path = "../fuels/tests/scripts/basic_script/out/debug/basic_script.bin";
//     let instance = bimbam_script::new(predicate.clone(), bin_path);
//
//     let a = 1000u64;
//     let b = 2000u32;
//
//
//     let coin = FuelInput::coin_signed(
//         Default::default(),
//         Default::default(),
//         10,
//         Default::default(),
//         Default::default(),
//         0,
//         0,
//     );
//
//     let c = Input::resource_signed(coin.into(), 0);
//
//
//     let message = FuelInput::message_signed(
//         Default::default(),
//         Default::default(),
//         Default::default(),
//         10,
//         0,
//         0,
//         vec![],
//     );
//
//     let zeroes = Bytes32::zeroed();
//
//     let contract_id = Contract::deploy(
//         "tests/contracts/contract_test/out/debug/contract_test.bin",
//         &predicate,
//         TxParameters::default(),
//         StorageConfiguration::default(),
//     )
//     .await?;
//
//     let contract_input = Input::contract(
//         UtxoId::new(zeroes, 0),
//         zeroes,
//         zeroes,
//         TxPointer::default(),
//         contract_id.into(),
//     );
//
//     let contract_output = Output::contract(0, zeroes, zeroes);
//
//     let result = instance
//         .main(a, b)
//         .with_inputs(vec![coin, message, contract_input])
//         .with_outputs(vec![contract_output])
//         .call()
//         .await?;
//
//     assert_eq!(result.value, "hello");
//
//     let parameters = TxParameters {
//         gas_price: 1,
//         gas_limit: 10000,
//         ..Default::default()
//     };
//
//     let result = instance.main(a, b).tx_params(parameters).call().await?;
//
//     assert_eq!(result.value, "hello");
//
//     Ok(())
// }

#[tokio::test]
async fn pay_with_predicate_vector_data() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), predicate.clone());
    let tx_params = TxParameters::new(Some(1000000), Some(10000), None);
    //
    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        192
    );

    let _call_params = CallParameters::new(Some(100), None, None);

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(tx_params)
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);
    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        186
    );

    Ok(())
}

#[tokio::test]
async fn pay_with_predicate_coins_messages_vectors() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
         Predicate(
            name="MyPredicate",
            abi="packages/fuels/tests/predicates/predicate_vectors/out/debug/predicate_vectors-abi.json"
        )
    );

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

    let mut predicate: Predicate = MyPredicate::load_from(
        "tests/predicates/predicate_vectors/out/debug/predicate_vectors.bin",
    )?
    .set_data(
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
    .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 30;
    let (provider, _, _, _, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), predicate.clone());
    let tx_params = TxParameters::new(Some(1000000), Some(10000), None);
    //
    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        360
    );

    let _call_params = CallParameters::new(Some(100), None, None);

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .tx_params(tx_params)
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);
    assert_eq!(
        *predicate
            .get_balances()
            .await?
            .get(format!("{:#?}", AssetId::default()).as_str())
            .unwrap(),
        341
    );

    Ok(())
}

#[tokio::test]
async fn predicate_contract_transfer() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 300;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_balances = predicate
        .provider()?
        .get_contract_balances(&contract_id)
        .await?;
    assert!(contract_balances.is_empty());

    let amount = 300;
    let (_tx_id, _receipts) = predicate
        .force_transfer_to_contract(
            &contract_id,
            amount,
            AssetId::default(),
            TxParameters::default(),
        )
        .await?;

    let contract_balances = predicate
        .provider()?
        .get_contract_balances(&contract_id)
        .await?;
    assert_eq!(contract_balances.len(), 1);

    let random_asset_balance = contract_balances
        .get(format!("{:#?}", AssetId::default()).as_str())
        .unwrap();
    assert_eq!(*random_asset_balance, 300);

    Ok(())
}

#[tokio::test]
async fn predicate_transfer_to_base_layer() -> Result<()> {
    use std::str::FromStr;

    use fuels::prelude::*;

    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 4;
    let num_messages = 8;
    let amount = 300;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let amount = 1000;
    let base_layer_address =
        Address::from_str("0x4710162c2e3a95a6faff05139150017c9e38e5e280432d546fae345d6ce6d8fe")
            .expect("Invalid address.");
    let base_layer_address = Bech32Address::from(base_layer_address);

    let (tx_id, msg_id, _receipts) = predicate
        .withdraw_to_base_layer(&base_layer_address, amount, TxParameters::default())
        .await?;

    let proof = predicate
        .provider()?
        .get_message_proof(&tx_id, &msg_id)
        .await?
        .expect("Failed to retrieve message proof.");

    assert_eq!(proof.amount, amount);
    assert_eq!(proof.recipient, base_layer_address);
    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn contract_tx_and_call_params_with_predicate() -> Result<()> {
    use fuels::prelude::*;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1_000_000_000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let balance = predicate.get_balances().await?;

    let contract_id = Contract::deploy(
        "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {contract_id}");
    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let my_tx_params = TxParameters::new(None, Some(1_000_000), None);

    let response = contract_methods
        .initialize_counter(42) // Our contract method.
        .tx_params(my_tx_params) // Chain the tx params setting method.
        .call() // Perform the contract call.
        .await?; // This is an async call, `.await` for it.

    let response = contract_methods
        .initialize_counter(42)
        .tx_params(TxParameters::default())
        .call()
        .await?;

    let my_tx_params = TxParameters::new(None, Some(1_000_000), None);

    let response = contract_methods
        .initialize_counter(42) // Our contract method.
        .tx_params(my_tx_params) // Chain the tx params setting method.
        .call() // Perform the contract call.
        .await?; // This is an async call, `.await` for it.

    let contract_methods = MyContract::new(contract_id, predicate.clone()).methods();

    let tx_params = TxParameters::default();

    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = contract_methods
        .get_msg_amount() // Our contract method.
        .tx_params(tx_params) // Chain the tx params setting method.
        .call_params(call_params)? // Chain the call params setting method.
        .call() // Perform the contract call.
        .await?;

    // Todo ContractCall add,   asset transfer append_custom_asset

    let response = contract_methods
        .initialize_counter(42)
        .call_params(CallParameters::default())?
        .call()
        .await?;

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn diff_asset_predicate_payment() -> Result<()> {
    use fuels::prelude::*;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let mut predicate: Predicate =
        MyPredicate::load_from("tests/predicates/predicate_vector/out/debug/predicate_vector.bin")?
            .set_data(2, 4, vec![2, 4, 42])
            .get_predicate();

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1_000_000_000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::deploy(
        "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let call_params = CallParameters::new(Some(1_000_000), Some(AssetId::from([1u8; 32])), None);

    let response = contract_methods
        .get_msg_amount() // Our contract method.
        .call_params(call_params)? // Chain the call params setting method.
        .call() // Perform the contract call.
        .await?;

    Ok(())
}
