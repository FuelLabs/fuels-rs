use std::default::Default;

use fuels::{
    core::{
        codec::{ABIEncoder, EncoderConfig},
        traits::Tokenizable,
    },
    prelude::*,
    programs::executable::Executable,
    types::{coin::Coin, coin_type::CoinType, input::Input, message::Message, output::Output},
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
    start_nonce: u64,
) -> (Vec<Coin>, Vec<Message>, AssetId) {
    let asset_id = AssetId::zeroed();
    let coins = setup_single_asset_coins(address, asset_id, num_coins, amount);
    let messages = (0..num_messages)
        .map(|i| {
            setup_single_message(
                &Bech32Address::default(),
                address,
                amount,
                (start_nonce + i).into(),
                vec![],
            )
        })
        .collect();

    (coins, messages, asset_id)
}

fn get_test_message_w_data(address: &Bech32Address, amount: u64, nonce: u64) -> Message {
    setup_single_message(
        &Bech32Address::default(),
        address,
        amount,
        nonce.into(),
        vec![1, 2, 3],
    )
}

// Setup function used to assign coins and messages to a predicate address
// and create a `receiver` wallet
async fn setup_predicate_test(
    predicate_address: &Bech32Address,
    num_coins: u64,
    num_messages: u64,
    amount: u64,
) -> Result<(Provider, u64, WalletUnlocked, u64, AssetId, WalletUnlocked)> {
    let receiver_num_coins = 1;
    let receiver_amount = 1;
    let receiver_balance = receiver_num_coins * receiver_amount;

    let predicate_balance = (num_coins + num_messages) * amount;
    let mut receiver = WalletUnlocked::new_random(None);
    let mut extra_wallet = WalletUnlocked::new_random(None);

    let (mut coins, messages, asset_id) =
        get_test_coins_and_messages(predicate_address, num_coins, num_messages, amount, 0);

    coins.extend(setup_single_asset_coins(
        receiver.address(),
        asset_id,
        receiver_num_coins,
        receiver_amount,
    ));
    coins.extend(setup_single_asset_coins(
        extra_wallet.address(),
        AssetId::zeroed(),
        10000,
        u64::MAX,
    ));

    coins.extend(setup_single_asset_coins(
        predicate_address,
        AssetId::from([1u8; 32]),
        num_coins,
        amount,
    ));

    let provider = setup_test_provider(coins, messages, None, None).await?;
    receiver.set_provider(provider.clone());
    extra_wallet.set_provider(provider.clone());

    Ok((
        provider,
        predicate_balance,
        receiver,
        receiver_balance,
        asset_id,
        extra_wallet,
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
        get_test_coins_and_messages(wallet.address(), num_coins, num_messages, amount, 0);

    let provider = setup_test_provider(coins, messages, None, None).await?;

    wallet.set_provider(provider.clone());

    let predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_provider(provider.clone());

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    wallet
        .transfer(
            predicate.address(),
            total_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    // The predicate has received the funds
    assert_address_balance(
        predicate.address(),
        &provider,
        asset_id,
        total_balance - expected_fee,
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn spend_predicate_coins_messages_basic() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(4097, 4097)?;

    let mut predicate: Predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    predicate
        .transfer(
            receiver.address(),
            predicate_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance - expected_fee,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn pay_with_predicate() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi = "e2e/sway/types/predicates/u64/out/release/u64-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::default().encode_data(32768)?;

    let mut predicate: Predicate =
        Predicate::load_from("sway/types/predicates/u64/out/release/u64.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();
    let tx_policies = TxPolicies::default()
        .with_tip(1)
        .with_script_gas_limit(1_000_000);

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    assert_eq!(
        predicate
            .get_asset_balance(provider.base_asset_id())
            .await?,
        192 - expected_fee
    );

    let response = contract_methods
        .initialize_counter(42) // Build the ABI call
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    assert_eq!(42, response.value);
    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 2;
    assert_eq!(
        predicate
            .get_asset_balance(provider.base_asset_id())
            .await?,
        191 - expected_fee
    );

    Ok(())
}

#[tokio::test]
async fn pay_with_predicate_vector_data() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi =
                "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::default().encode_data(12, 30, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();
    let tx_policies = TxPolicies::default()
        .with_tip(1)
        .with_script_gas_limit(1_000_000);

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    assert_eq!(
        predicate
            .get_asset_balance(provider.base_asset_id())
            .await?,
        192 - expected_fee
    );

    let response = contract_methods
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 2;
    assert_eq!(42, response.value);
    assert_eq!(
        predicate
            .get_asset_balance(provider.base_asset_id())
            .await?,
        191 - expected_fee
    );

    Ok(())
}

#[tokio::test]
async fn predicate_contract_transfer() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(2, 40, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 300;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;

    let contract_balances = provider.get_contract_balances(&contract_id).await?;
    assert!(contract_balances.is_empty());

    let amount = 300;
    predicate
        .force_transfer_to_contract(
            &contract_id,
            amount,
            AssetId::zeroed(),
            TxPolicies::default(),
        )
        .await?;

    let contract_balances = predicate
        .try_provider()?
        .get_contract_balances(&contract_id)
        .await?;
    assert_eq!(contract_balances.len(), 1);

    let random_asset_balance = contract_balances.get(&AssetId::zeroed()).unwrap();
    assert_eq!(*random_asset_balance, 300);

    Ok(())
}

#[tokio::test]
async fn predicate_transfer_to_base_layer() -> Result<()> {
    use std::str::FromStr;

    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(22, 20, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 300;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let amount = 1000;
    let base_layer_address =
        Address::from_str("0x4710162c2e3a95a6faff05139150017c9e38e5e280432d546fae345d6ce6d8fe")?;
    let base_layer_address = Bech32Address::from(base_layer_address);

    let (tx_id, msg_nonce, _receipts) = predicate
        .withdraw_to_base_layer(&base_layer_address, amount, TxPolicies::default())
        .await?;

    // Create the next commit block to be able generate the proof
    provider.produce_blocks(1, None).await?;

    let proof = predicate
        .try_provider()?
        .get_message_proof(&tx_id, &msg_nonce, None, Some(2))
        .await?
        .expect("failed to retrieve message proof");

    assert_eq!(proof.amount, amount);
    assert_eq!(proof.recipient, base_layer_address);

    Ok(())
}

#[tokio::test]
async fn predicate_transfer_with_signed_resources() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(2, 40, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let predicate_num_coins = 4;
    let predicate_num_messages = 3;
    let predicate_amount = 1000;
    let predicate_balance = (predicate_num_coins + predicate_num_messages) * predicate_amount;

    let mut wallet = WalletUnlocked::new_random(None);
    let wallet_num_coins = 4;
    let wallet_num_messages = 3;
    let wallet_amount = 1000;
    let wallet_balance = (wallet_num_coins + wallet_num_messages) * wallet_amount;

    let (mut coins, mut messages, asset_id) = get_test_coins_and_messages(
        predicate.address(),
        predicate_num_coins,
        predicate_num_messages,
        predicate_amount,
        0,
    );
    let (wallet_coins, wallet_messages, _) = get_test_coins_and_messages(
        wallet.address(),
        wallet_num_coins,
        wallet_num_messages,
        wallet_amount,
        predicate_num_messages,
    );

    coins.extend(wallet_coins);
    messages.extend(wallet_messages);

    let provider = setup_test_provider(coins, messages, None, None).await?;
    wallet.set_provider(provider.clone());
    predicate.set_provider(provider.clone());

    let mut inputs = wallet
        .get_asset_inputs_for_amount(asset_id, wallet_balance, None)
        .await?;
    let predicate_inputs = predicate
        .get_asset_inputs_for_amount(asset_id, predicate_balance, None)
        .await?;
    inputs.extend(predicate_inputs);

    let outputs = vec![Output::change(predicate.address().into(), 0, asset_id)];

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, Default::default());
    tb.add_signer(wallet.clone())?;

    let tx = tb.build(&provider).await?;

    provider.send_transaction_and_await_commit(tx).await?;

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    assert_address_balance(
        predicate.address(),
        &provider,
        asset_id,
        predicate_balance + wallet_balance - expected_fee,
    )
    .await;

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn contract_tx_and_call_params_with_predicate() -> Result<()> {
    use fuels::prelude::*;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi =
                "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::default().encode_data(22, 20, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "./sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;
    println!("Contract deployed @ {contract_id}");

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let tx_policies = TxPolicies::default().with_tip(100);

    let call_params_amount = 100;
    let call_params = CallParameters::default()
        .with_amount(call_params_amount)
        .with_asset_id(AssetId::zeroed());

    {
        let response = contract_methods
            .get_msg_amount()
            .with_tx_policies(tx_policies)
            .call_params(call_params.clone())?
            .call()
            .await?;

        // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
        let expected_fee = 2;
        assert_eq!(
            predicate.get_asset_balance(&AssetId::zeroed()).await?,
            1800 - expected_fee
        );
    }
    {
        let custom_asset = AssetId::from([1u8; 32]);

        let response = contract_methods
            .get_msg_amount()
            .call_params(call_params)?
            .add_custom_asset(custom_asset, 100, Some(Bech32Address::default()))
            .call()
            .await?;

        assert_eq!(predicate.get_asset_balance(&custom_asset).await?, 900);
    }

    Ok(())
}

#[tokio::test]
#[allow(unused_variables)]
async fn diff_asset_predicate_payment() -> Result<()> {
    use fuels::prelude::*;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi =
                "e2e/sway/types/predicates/predicate_vector/out/release/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::default().encode_data(28, 14, vec![2, 4, 42])?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1_000_000_000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "./sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let call_params = CallParameters::default()
        .with_amount(1_000_000)
        .with_asset_id(AssetId::from([1u8; 32]));

    let response = contract_methods
        .get_msg_amount()
        .call_params(call_params)?
        .call()
        .await?;

    Ok(())
}

#[tokio::test]
async fn predicate_default_configurables() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_configurables/out/release/predicate_configurables-abi.json"
    ));

    let new_struct = StructWithGeneric {
        field_1: 8u8,
        field_2: 16,
    };
    let new_enum = EnumWithGeneric::VariantOne(true);

    let predicate_data = MyPredicateEncoder::default().encode_data(
        true,
        8,
        (8, true),
        [253, 254, 255],
        new_struct,
        new_enum,
    )?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/predicates/predicate_configurables/out/release/predicate_configurables.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    predicate
        .transfer(
            receiver.address(),
            predicate_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance - expected_fee,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn predicate_configurables() -> Result<()> {
    // ANCHOR: predicate_configurables
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_configurables/out/release/predicate_configurables-abi.json"
    ));

    let new_tuple = (16, false);
    let new_array = [123, 124, 125];
    let new_struct = StructWithGeneric {
        field_1: 32u8,
        field_2: 64,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyPredicateConfigurables::default()
        .with_U8(8)?
        .with_TUPLE(new_tuple)?
        .with_ARRAY(new_array)?
        .with_STRUCT(new_struct.clone())?
        .with_ENUM(new_enum.clone())?;

    let predicate_data = MyPredicateEncoder::default()
        .encode_data(true, 8u8, new_tuple, new_array, new_struct, new_enum)?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/predicates/predicate_configurables/out/release/predicate_configurables.bin",
    )?
    .with_data(predicate_data)
    .with_configurables(configurables);
    // ANCHOR_END: predicate_configurables

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    predicate
        .transfer(
            receiver.address(),
            predicate_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance - expected_fee,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn predicate_adjust_fee_persists_message_w_data() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(4097, 4097)?;

    let mut predicate: Predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_data(predicate_data);

    let amount = 1000;
    let coins = setup_single_asset_coins(predicate.address(), AssetId::zeroed(), 1, amount);
    let message = get_test_message_w_data(predicate.address(), amount, Default::default());
    let message_input = Input::resource_predicate(
        CoinType::Message(message.clone()),
        predicate.code().to_vec(),
        predicate.data().to_vec(),
    );

    let provider = setup_test_provider(coins, vec![message.clone()], None, None).await?;
    predicate.set_provider(provider.clone());

    let mut tb = ScriptTransactionBuilder::prepare_transfer(
        vec![message_input.clone()],
        vec![],
        TxPolicies::default().with_tip(1),
    );
    predicate.adjust_for_fee(&mut tb, 1000).await?;
    let tx = tb.build(&provider).await?;

    assert_eq!(tx.inputs().len(), 2);
    assert_eq!(tx.inputs()[0].message_id().unwrap(), message.message_id());

    Ok(())
}

#[tokio::test]
async fn predicate_transfer_non_base_asset() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(32, 32)?;

    let mut predicate: Predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_data(predicate_data);

    let mut wallet = WalletUnlocked::new_random(None);

    let amount = 5;
    let non_base_asset_id = AssetId::new([1; 32]);

    // wallet has base and predicate non base asset
    let mut coins = setup_single_asset_coins(wallet.address(), AssetId::zeroed(), 1, amount);
    coins.extend(setup_single_asset_coins(
        predicate.address(),
        non_base_asset_id,
        1,
        amount,
    ));

    let provider = setup_test_provider(coins, vec![], None, None).await?;
    predicate.set_provider(provider.clone());
    wallet.set_provider(provider.clone());

    let inputs = predicate
        .get_asset_inputs_for_amount(non_base_asset_id, amount, None)
        .await?;
    let outputs = vec![
        Output::change(wallet.address().into(), 0, non_base_asset_id),
        Output::change(wallet.address().into(), 0, *provider.base_asset_id()),
    ];

    let mut tb = ScriptTransactionBuilder::prepare_transfer(
        inputs,
        outputs,
        TxPolicies::default().with_tip(1),
    );

    tb.add_signer(wallet.clone())?;
    wallet.adjust_for_fee(&mut tb, 0).await?;

    let tx = tb.build(&provider).await?;

    provider
        .send_transaction_and_await_commit(tx)
        .await?
        .check(None)?;

    let wallet_balance = wallet.get_asset_balance(&non_base_asset_id).await?;

    assert_eq!(wallet_balance, amount);

    Ok(())
}

#[tokio::test]
async fn predicate_can_access_manually_added_witnesses() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_witnesses/out/release/predicate_witnesses-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(0, 1)?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/predicates/predicate_witnesses/out/release/predicate_witnesses.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 0;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let amount_to_send = 12;
    let inputs = predicate
        .get_asset_inputs_for_amount(asset_id, amount_to_send, None)
        .await?;
    let outputs =
        predicate.get_asset_outputs_for_amount(receiver.address(), asset_id, amount_to_send);

    let mut tx = ScriptTransactionBuilder::prepare_transfer(
        inputs,
        outputs,
        TxPolicies::default().with_witness_limit(32),
    )
    .build(&provider)
    .await?;

    let witness = ABIEncoder::default().encode(&[64u64.into_token()])?; // u64 because this is VM memory
    let witness2 = ABIEncoder::default().encode(&[4096u64.into_token()])?;

    tx.append_witness(witness.into())?;
    tx.append_witness(witness2.into())?;

    provider.send_transaction_and_await_commit(tx).await?;

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    // The predicate has spent the funds
    assert_address_balance(
        predicate.address(),
        &provider,
        asset_id,
        predicate_balance - amount_to_send - expected_fee,
    )
    .await;

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

#[tokio::test]
async fn tx_id_not_changed_after_adding_witnesses() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_witnesses/out/release/predicate_witnesses-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(0, 1)?;

    let mut predicate: Predicate = Predicate::load_from(
        "sway/predicates/predicate_witnesses/out/release/predicate_witnesses.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 0;
    let amount = 16;
    let (provider, _predicate_balance, receiver, _receiver_balance, asset_id, _) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let amount_to_send = 12;
    let inputs = predicate
        .get_asset_inputs_for_amount(asset_id, amount_to_send, None)
        .await?;
    let outputs =
        predicate.get_asset_outputs_for_amount(receiver.address(), asset_id, amount_to_send);

    let mut tx = ScriptTransactionBuilder::prepare_transfer(
        inputs,
        outputs,
        TxPolicies::default().with_witness_limit(32),
    )
    .build(&provider)
    .await?;

    let tx_id = tx.id(provider.chain_id());

    let witness = ABIEncoder::default().encode(&[64u64.into_token()])?; // u64 because this is VM memory
    let witness2 = ABIEncoder::default().encode(&[4096u64.into_token()])?;

    tx.append_witness(witness.into())?;
    tx.append_witness(witness2.into())?;
    let tx_id_after_witnesses = tx.id(provider.chain_id());

    let tx_id_from_provider = provider.send_transaction(tx).await?;

    assert_eq!(tx_id, tx_id_after_witnesses);
    assert_eq!(tx_id, tx_id_from_provider);

    Ok(())
}

#[tokio::test]
async fn predicate_encoder_config_is_applied() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));
    {
        let _encoding_ok = MyPredicateEncoder::default()
            .encode_data(4097, 4097)
            .expect("should not fail as it uses the default encoder config");
    }
    {
        let encoder_config = EncoderConfig {
            max_tokens: 1,
            ..Default::default()
        };
        let encoding_error = MyPredicateEncoder::new(encoder_config)
            .encode_data(4097, 4097)
            .expect_err("should fail");

        assert!(encoding_error
            .to_string()
            .contains("token limit `1` reached while encoding"));
    }

    Ok(())
}

#[tokio::test]
async fn predicate_transfers_non_base_asset() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(4097, 4097)?;
    let mut predicate: Predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_message = 6;
    let amount = 20;
    let (provider, _, receiver, _, _, _) =
        setup_predicate_test(predicate.address(), num_coins, num_message, amount).await?;
    predicate.set_provider(provider);
    let other_asset_id = AssetId::from([1u8; 32]);

    let send_amount = num_coins * amount;
    predicate
        .transfer(
            receiver.address(),
            send_amount,
            other_asset_id,
            TxPolicies::default(),
        )
        .await?;

    assert_eq!(predicate.get_asset_balance(&other_asset_id).await?, 0,);

    assert_eq!(
        receiver.get_asset_balance(&other_asset_id).await?,
        send_amount,
    );

    Ok(())
}

#[tokio::test]
async fn predicate_with_invalid_data_fails() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::default().encode_data(0, 100)?;
    let mut predicate: Predicate =
        Predicate::load_from("sway/predicates/basic_predicate/out/release/basic_predicate.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_message = 6;
    let amount = 20;
    let (provider, _, receiver, _, _, _) =
        setup_predicate_test(predicate.address(), num_coins, num_message, amount).await?;
    predicate.set_provider(provider);
    let other_asset_id = AssetId::from([1u8; 32]);

    let send_amount = num_coins * amount;
    let error_string = predicate
        .transfer(
            receiver.address(),
            send_amount,
            other_asset_id,
            TxPolicies::default(),
        )
        .await
        .unwrap_err()
        .to_string();

    assert!(error_string.contains("PredicateVerificationFailed(Panic(PredicateReturnedNonOne))"));
    assert_eq!(receiver.get_asset_balance(&other_asset_id).await?, 0);

    Ok(())
}

#[tokio::test]
async fn predicate_blobs() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_blobs/out/release/predicate_blobs-abi.json"
    ));

    // ANCHOR: preparing_the_predicate
    let configurables = MyPredicateConfigurables::default().with_SECRET_NUMBER(10001)?;

    let predicate_data = MyPredicateEncoder::default().encode_data(1, 19)?;

    let executable =
        Executable::load_from("sway/predicates/predicate_blobs/out/release/predicate_blobs.bin")?;

    let loader = executable
        .convert_to_loader()?
        .with_configurables(configurables);

    let mut predicate: Predicate = Predicate::from_code(loader.code()).with_data(predicate_data);
    // ANCHOR_END: preparing_the_predicate

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, extra_wallet) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    // we don't want to pay with the recipient wallet so that we don't affect the assertion we're
    // gonna make later on
    // ANCHOR: uploading_the_blob
    loader.upload_blob(extra_wallet).await?;

    predicate.set_provider(provider.clone());

    let expected_fee = 1;
    predicate
        .transfer(
            receiver.address(),
            predicate_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;
    // ANCHOR_END: uploading_the_blob

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance - expected_fee,
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn predicate_configurables_in_blobs() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "e2e/sway/predicates/predicate_configurables/out/release/predicate_configurables-abi.json"
    ));

    let new_tuple = (16, false);
    let new_array = [123, 124, 125];
    let new_struct = StructWithGeneric {
        field_1: 32u8,
        field_2: 64,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyPredicateConfigurables::default()
        .with_U8(8)?
        .with_TUPLE(new_tuple)?
        .with_ARRAY(new_array)?
        .with_STRUCT(new_struct.clone())?
        .with_ENUM(new_enum.clone())?;

    let predicate_data = MyPredicateEncoder::default()
        .encode_data(true, 8u8, new_tuple, new_array, new_struct, new_enum)?;

    let executable = Executable::load_from(
        "sway/predicates/predicate_configurables/out/release/predicate_configurables.bin",
    )?;

    let loader = executable
        .convert_to_loader()?
        .with_configurables(configurables);

    let mut predicate: Predicate = Predicate::from_code(loader.code()).with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, predicate_balance, receiver, receiver_balance, asset_id, extra_wallet) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    loader.upload_blob(extra_wallet).await?;

    // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
    let expected_fee = 1;
    predicate
        .transfer(
            receiver.address(),
            predicate_balance - expected_fee,
            asset_id,
            TxPolicies::default(),
        )
        .await?;

    // The predicate has spent the funds
    assert_address_balance(predicate.address(), &provider, asset_id, 0).await;

    // Funds were transferred
    assert_address_balance(
        receiver.address(),
        &provider,
        asset_id,
        receiver_balance + predicate_balance - expected_fee,
    )
    .await;

    Ok(())
}
