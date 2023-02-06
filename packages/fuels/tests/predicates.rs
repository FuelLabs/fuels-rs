use fuels::{
    prelude::*,
    tx::AssetId,
    types::{coin::Coin, message::Message},
};

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
/*

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
    let amount = 100_000;
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
*/

#[tokio::test]
async fn pay_with_predicate() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        // Predicate(
        //     name = "MyPredicate",
        //     abi = "packages/fuels/tests/predicates/predicate_u64/out/debug/predicate_u64-abi.json"
        // )
    );

    use my_predicate_mod::MyPredicate;

    let mut predicate =
        MyPredicate::load_from("tests/predicates/predicate_u64/out/debug/predicate_u64.bin")?;

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // dbg!(&predicate.address());
    predicate.set_provider(Some(provider.clone()));
    // dbg!(&predicate.provider());

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &predicate,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
        .await?;

    let mut wallet = WalletUnlocked::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, vec![], None, None).await;
    let connected_provider = Provider::connect(address.to_string()).await?;

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance_connected = MyContract::new(contract_id.clone(), wallet.clone());

    let response = contract_instance_connected
        .methods()
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    assert_eq!(42, response.value);

    // wallet.set_provider(launched_provider);
    // let contract_instance_launched = MyContract::new(contract_id, wallet);
    //
    // let response = contract_instance_launched
    //     .methods()
    //     .increment_counter(10)
    //     .call()
    //     .await?;
    // assert_eq!(52, response.value);

    // assert!(false);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[no_implicit_prelude]
pub mod my_predicate_mod {
    use ::fuels::prelude::AssetId;
    use ::fuels_types::resource::Resource;
    use ::std::boxed::Box;
    use ::std::{
        clone::Clone,
        convert::{From, Into, TryFrom},
        format,
        iter::IntoIterator,
        iter::Iterator,
        marker::Sized,
        panic,
        string::ToString,
        vec,
    };

    #[cfg_attr(not(target_arch = "wasm32"), ::async_trait::async_trait)]
    impl ::fuels::signers::PayFee for MyPredicate {
        type Error = ::fuels::types::errors::Error;
        fn address(&self) -> &::fuels::prelude::Bech32Address {
            &self.address
        }
        async fn pay_fee_resources<
            'a_t,
            Tx: ::fuels::tx::Chargeable
                + ::fuels::tx::field::Inputs
                + ::fuels::tx::field::Outputs
                + ::std::marker::Send
                + ::fuels::tx::Cacheable
                + ::fuels::tx::UniqueIdentifier
                + ::fuels::tx::field::Witnesses,
        >(
            &'a_t self,
            tx: &'a_t mut Tx,
            previous_base_amount: u64,
        ) -> ::fuels::types::errors::Result<()> {
            // ::std::boxed::Box::pin(async move {
            let consensus_parameters = self
                .get_provider()?
                .chain_info()
                .await?
                .consensus_parameters;
            let transaction_fee =
                ::fuels::tx::TransactionFee::checked_from_tx(&consensus_parameters, tx)
                    .expect("Error calculating TransactionFee");
            let (base_asset_inputs, remaining_inputs): (::std::vec::Vec<_>, ::std::vec::Vec<_>) = tx.inputs().iter().cloned().partition(|input| { ::std::matches!(input , :: fuels :: tx :: Input :: MessageSigned { .. }) || ::std::matches!(input , :: fuels :: tx :: Input :: CoinSigned { asset_id , .. } if asset_id == & :: fuels :: core :: constants :: BASE_ASSET_ID) });
            let base_inputs_sum: u64 = base_asset_inputs
                .iter()
                .map(|input| input.amount().unwrap())
                .sum();
            if base_inputs_sum < previous_base_amount {
                return ::std::result::Result::Err(::fuels::types::errors::Error::WalletError(
                    ::std::format!(
                        "The provided base asset amount is less than the present input coins"
                    ),
                ));
            }
            let mut new_base_amount = transaction_fee.total() + previous_base_amount;
            let is_consuming_utxos = tx
                .inputs()
                .iter()
                .any(|input| !::std::matches!(input, ::fuels::tx::Input::Contract { .. }));
            const MIN_AMOUNT: u64 = 1;
            if !is_consuming_utxos && new_base_amount == 0 {
                new_base_amount = MIN_AMOUNT;
            }
            let new_base_inputs = self
                .get_asset_inputs_for_amount(
                    ::fuels::core::constants::BASE_ASSET_ID,
                    new_base_amount,
                    0,
                )
                .await?;
            let adjusted_inputs: ::std::vec::Vec<_> = remaining_inputs
                .into_iter()
                .chain(new_base_inputs.into_iter())
                .collect();
            ::std::dbg!(&adjusted_inputs);
            *tx.inputs_mut() = adjusted_inputs;
            let is_base_change_present = tx.outputs().iter().any(|output| { ::std::matches!(output , :: fuels :: tx :: Output :: Change { asset_id , .. } if asset_id == & :: fuels :: core :: constants :: BASE_ASSET_ID) });
            if !is_base_change_present && new_base_amount != 0 {
                tx.outputs_mut().push(::fuels::tx::Output::change(
                    self.address().into(),
                    0,
                    ::fuels::core::constants::BASE_ASSET_ID,
                ));
            }
            ::std::result::Result::Ok(())
            // }).await
        }
        fn get_provider(
            &self,
        ) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
            self.provider()
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), ::async_trait::async_trait)]
    impl ::fuels::signers::Account for MyPredicate {
        type Error = ::fuels::types::errors::Error;

        fn address(&self) -> &::fuels::types::bech32::Bech32Address {
            &self.address
        }

        fn get_provider(
            &self,
        ) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
            self.provider()
        }

        fn set_provider(&mut self, provider: ::fuels::signers::provider::Provider) {
            self.set_provider(::std::option::Option::Some(provider))
        }

        async fn get_spendable_resources(
            &self,
            asset_id: AssetId,
            amount: u64,
        ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::types::resource::Resource>>
        {
            self.provider()?
                .get_spendable_resources(&self.address, asset_id, amount)
                .await
                .map_err(::std::convert::Into::into)
        }
    }

    #[derive(Debug, Clone)]
    pub struct MyPredicate {
        address: ::fuels::types::bech32::Bech32Address,
        code: ::std::vec::Vec<u8>,
        data: ::fuels::core::abi_encoder::UnresolvedBytes,
        provider: ::std::option::Option<::fuels::prelude::Provider>,
    }

    impl MyPredicate {
        pub fn new(code: ::std::vec::Vec<u8>) -> Self {
            let address: ::fuels::types::Address =
                (*::fuels::tx::Contract::root_from_code(&code)).into();
            Self {
                address: address.clone().into(),
                code,
                data: ::fuels::core::abi_encoder::UnresolvedBytes::new(),
                provider: ::std::option::Option::None,
            }
        }
        pub fn load_from(file_path: &str) -> ::fuels::types::errors::Result<Self> {
            ::std::result::Result::Ok(Self::new(::std::fs::read(file_path)?))
        }
        pub fn address(&self) -> &::fuels::types::bech32::Bech32Address {
            &self.address
        }
        pub fn code(&self) -> ::std::vec::Vec<u8> {
            self.code.clone()
        }
        pub fn provider(
            &self,
        ) -> ::fuels::types::errors::Result<&::fuels::signers::provider::Provider> {
            self.provider
                .as_ref()
                .ok_or(::fuels::types::errors::Error::from(
                    ::fuels::signers::wallet::WalletError::NoProvider,
                ))
        }
        pub fn set_provider(
            &mut self,
            provider: ::std::option::Option<::fuels::prelude::Provider>,
        ) {
            self.provider = provider
        }
        pub fn data(&self) -> ::fuels::core::abi_encoder::UnresolvedBytes {
            self.data.clone()
        }
        pub async fn receive(
            &self,
            from: &::fuels::signers::wallet::WalletUnlocked,
            amount: u64,
            asset_id: ::fuels::types::AssetId,
            tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>,
        ) -> ::fuels::types::errors::Result<(
            ::std::string::String,
            ::std::vec::Vec<::fuels::tx::Receipt>,
        )> {
            let tx_parameters = tx_parameters.unwrap_or_default();
            from.transfer(self.address(), amount, asset_id, tx_parameters)
                .await
        }
        pub async fn spend(
            &self,
            to: &::fuels::signers::wallet::WalletUnlocked,
            amount: u64,
            asset_id: ::fuels::types::AssetId,
            tx_parameters: ::std::option::Option<::fuels::core::parameters::TxParameters>,
        ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::tx::Receipt>> {
            let tx_parameters = tx_parameters.unwrap_or_default();
            to.receive_from_predicate(
                self.address(),
                self.code(),
                amount,
                asset_id,
                self.data(),
                tx_parameters,
            )
            .await
        }
        pub async fn get_asset_inputs_for_amount(
            &self,
            asset_id: ::fuels::types::AssetId,
            amount: u64,
            witness_index: u8,
        ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::tx::Input>> {
            ::std::result::Result::Ok(
                self.get_spendable_resources(asset_id, amount)
                    .await?
                    .into_iter()
                    .map(|resource| match resource {
                        ::fuels::types::resource::Resource::Coin(coin) => {
                            self.create_coin_input(coin, asset_id, witness_index)
                        }
                        ::fuels::types::resource::Resource::Message(message) => {
                            self.create_message_input(message, witness_index)
                        }
                    })
                    .collect::<::std::vec::Vec<::fuels::tx::Input>>(),
            )
        }
        pub async fn get_spendable_resources(
            &self,
            asset_id: ::fuels::types::AssetId,
            amount: u64,
        ) -> ::fuels::types::errors::Result<::std::vec::Vec<::fuels::types::resource::Resource>>
        {
            self.provider()?
                .get_spendable_resources(&self.address, asset_id, amount)
                .await
                .map_err(::std::convert::Into::into)
        }
        fn create_coin_input(
            &self,
            coin: ::fuels::types::coin::Coin,
            asset_id: ::fuels::types::AssetId,
            witness_index: u8,
        ) -> ::fuels::tx::Input {
            ::fuels::tx::Input::coin_signed(
                coin.utxo_id,
                coin.owner.into(),
                coin.amount,
                asset_id,
                ::fuels::tx::TxPointer::new(0, 0),
                witness_index,
                0,
            )
        }
        fn create_message_input(
            &self,
            message: ::fuels::types::message::Message,
            witness_index: u8,
        ) -> ::fuels::tx::Input {
            ::fuels::tx::Input::message_signed(
                message.message_id(),
                message.sender.into(),
                message.recipient.into(),
                message.amount,
                message.nonce,
                witness_index,
                message.data,
            )
        }
        #[doc = "Run the predicate's encode function with the provided arguments"]
        pub fn encode_data(&self, a: u64) -> Self {
            let data = ::fuels::core::abi_encoder::ABIEncoder::encode(&[
                ::fuels::types::traits::Tokenizable::into_token(a),
            ])
            .expect("Cannot encode predicate data");
            Self {
                address: self.address.clone(),
                code: self.code.clone(),
                data,
                provider: self.provider.clone(),
            }
        }
    }
}
