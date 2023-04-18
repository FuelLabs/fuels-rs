use fuel_tx::Output;
use fuels::{
    prelude::*,
    types::AssetId,
    types::{coin::Coin, message::Message},
};
use fuels_accounts::{predicate::Predicate, Account};
use fuels_types::transaction_builders::{ScriptTransactionBuilder, TransactionBuilder};

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

    let mut predicate =
        Predicate::load_from("tests/predicates/basic_predicate/out/debug/basic_predicate.bin")?;

    predicate.set_provider(provider.clone());

    wallet
        .transfer(
            predicate.address(),
            total_balance,
            asset_id,
            TxParameters::default(),
        )
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

    let predicate_data = MyPredicateEncoder::encode_data(4097, 4097);

    let mut predicate: Predicate =
        Predicate::load_from("tests/predicates/basic_predicate/out/debug/basic_predicate.bin")?
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
async fn pay_with_predicate() -> Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi = "packages/fuels/tests/types/predicates/u64/out/debug/u64-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::encode_data(32768);

    let mut predicate: Predicate =
        Predicate::load_from("tests/types/predicates/u64/out/debug/u64.bin")?
            .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxParameters::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();
    let tx_params = TxParameters::new(1000000, 10000, 0);

    assert_eq!(predicate.get_asset_balance(&BASE_ASSET_ID).await?, 192);

    let response = contract_methods
        .initialize_counter(42) // Build the ABI call
        .tx_params(tx_params)
        .call()
        .await?;

    assert_eq!(42, response.value);
    assert_eq!(predicate.get_asset_balance(&BASE_ASSET_ID).await?, 187);

    Ok(())
}

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
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 4;
    let num_messages = 8;
    let amount = 16;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxParameters::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();
    let tx_params = TxParameters::default()
        .set_gas_price(100000)
        .set_gas_limit(10000);

    assert_eq!(predicate.get_asset_balance(&BASE_ASSET_ID).await?, 192);

    let response = contract_methods
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    assert_eq!(42, response.value);
    assert_eq!(predicate.get_asset_balance(&BASE_ASSET_ID).await?, 190);

    Ok(())
}

#[tokio::test]
async fn predicate_contract_transfer() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
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
    let amount = 300;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxParameters::default())
    .await?;

    let contract_balances = predicate
        .try_provider()?
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
        .try_provider()?
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
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(predicate_data);

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
        .try_provider()?
        .get_message_proof(&tx_id, &msg_id)
        .await?
        .expect("Failed to retrieve message proof.");

    assert_eq!(proof.amount, amount);
    assert_eq!(proof.recipient, base_layer_address);
    Ok(())
}

#[tokio::test]
async fn predicate_transfer_with_signed_resources() -> Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
    ));

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
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
    );
    let (wallet_coins, wallet_messages, _) = get_test_coins_and_messages(
        wallet.address(),
        wallet_num_coins,
        wallet_num_messages,
        wallet_amount,
    );

    coins.extend(wallet_coins);
    messages.extend(wallet_messages);

    let (provider, _address) = setup_test_provider(coins, messages, None, None).await;
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

    let params = provider.consensus_parameters().await?;
    let mut tx = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, Default::default())
        .set_consensus_parameters(params)
        .build()?;
    wallet.sign_transaction(&mut tx)?;

    provider.send_transaction(&tx).await?;

    assert_address_balance(
        predicate.address(),
        &provider,
        asset_id,
        predicate_balance + wallet_balance,
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
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxParameters::default())
    .await?;
    println!("Contract deployed @ {contract_id}");
    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let my_tx_params = TxParameters::default().set_gas_price(100);

    let call_params_amount = 100;
    let call_params = CallParameters::default()
        .set_amount(call_params_amount)
        .set_asset_id(AssetId::default());

    {
        let response = contract_methods
            .get_msg_amount()
            .tx_params(my_tx_params)
            .call_params(call_params.clone())?
            .call()
            .await?;

        assert_eq!(
            predicate.get_asset_balance(&AssetId::default()).await?,
            1899
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
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ),
        Predicate(
        name = "MyPredicate",
        abi =
            "packages/fuels/tests/types/predicates/predicate_vector/out/debug/predicate_vector-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::encode_data(2, 4, vec![2, 4, 42]);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/types/predicates/predicate_vector/out/debug/predicate_vector.bin",
    )?
    .with_data(predicate_data);

    let num_coins = 1;
    let num_messages = 1;
    let amount = 1_000_000_000;
    let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id) =
        setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

    predicate.set_provider(provider.clone());

    let contract_id = Contract::load_from(
        "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy(&predicate, TxParameters::default())
    .await?;

    let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();

    let call_params = CallParameters::default()
        .set_amount(1_000_000)
        .set_asset_id(AssetId::from([1u8; 32]));

    let response = contract_methods
        .get_msg_amount()
        .call_params(call_params)?
        .call()
        .await?;

    Ok(())
}

#[tokio::test]
async fn predicate_configurables() -> Result<()> {
    // ANCHOR: predicate_configurables
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "packages/fuels/tests/predicates/predicate_configurables/out/debug/predicate_configurables-abi.json"
    ));

    let new_struct = StructWithGeneric {
        field_1: 32u8,
        field_2: 64,
    };
    let new_enum = EnumWithGeneric::VariantTwo;

    let configurables = MyPredicateConfigurables::new()
        .set_STRUCT(new_struct.clone())
        .set_ENUM(new_enum.clone());

    let predicate_data = MyPredicateEncoder::encode_data(8u8, true, new_struct, new_enum);

    let mut predicate: Predicate = Predicate::load_from(
        "tests/predicates/predicate_configurables/out/debug/predicate_configurables.bin",
    )?
    .with_data(predicate_data)
    .with_configurables(configurables);
    // ANCHOR_END: predicate_configurables

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
