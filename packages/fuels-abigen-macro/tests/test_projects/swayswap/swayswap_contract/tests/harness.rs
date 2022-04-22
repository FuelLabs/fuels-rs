use fuel_tx::{AssetId, Salt};
use fuels_abigen_macro::abigen;
use fuels_contract::{
    contract::Contract,
    parameters::{CallParameters, TxParameters},
};
use fuels_core::constants::NATIVE_ASSET_ID;
use fuels_signers::{util::test_helpers, Signer};

#[tokio::test]
async fn swayswap() {
    // Provider and Wallet
    let (provider, wallet) = test_helpers::setup_test_provider_and_wallet().await;

    /////////////////////////////
    // Load the Swayswap contract
    /////////////////////////////
    abigen!(TestSwayswap, "out/debug/swayswap_contract-abi.json");
    let swayswap_salt = Salt::from([0u8; 32]);
    let swayswap_compiled =
        Contract::load_sway_contract("out/debug/swayswap_contract.bin", swayswap_salt).unwrap();

    // Get the contract ID and a handle to it
    let swayswap_contract_id = Contract::deploy(
        &swayswap_compiled,
        &provider.clone(),
        &wallet.clone(),
        TxParameters::default(),
    )
    .await
    .unwrap();
    let swayswap_instance = TestSwayswap::new(
        swayswap_contract_id.to_string(),
        provider.clone(),
        wallet.clone(),
    );

    // Depost some native assets
    swayswap_instance
        .deposit()
        .call_params(CallParameters::new(Some(11), None))
        .call()
        .await
        .unwrap();

    // Withdraw some native assets
    let native_asset_id = testswayswap_mod::ContractId {
        value: NATIVE_ASSET_ID,
    };
    swayswap_instance
        .withdraw(11, native_asset_id)
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    //////////////////////////
    // Load the Token contract
    //////////////////////////
    abigen!(
        TestToken,
        "../token_contract/out/debug/token_contract-abi.json"
    );
    let token_salt = Salt::from([0u8; 32]);
    let token_compiled =
        Contract::load_sway_contract("../token_contract/out/debug/token_contract.bin", token_salt)
            .unwrap();

    // Get the contract ID and a handle to it
    let token_contract_id =
        Contract::deploy(&token_compiled, &provider, &wallet, TxParameters::default())
            .await
            .unwrap();
    let token_instance = TestToken::new(token_contract_id.to_string(), provider, wallet.clone());

    // Mint some alt tokens
    token_instance.mint_coins(10000, 1).call().await.unwrap();

    // Check the balance of the contract of its own asset
    let target = testtoken_mod::ContractId {
        value: token_contract_id.into(),
    };
    let token_asset_id = testtoken_mod::ContractId {
        value: token_contract_id.into(),
    };
    let result = token_instance
        .get_balance(target.clone(), token_asset_id.clone())
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 10000);

    //////////////////////////////////////////
    // Start transferring and adding liquidity
    //////////////////////////////////////////

    // Transfer some alt tokens to the wallet
    let address = wallet.address();
    let address = testtoken_mod::Address {
        value: address.into(),
    };
    token_instance
        .transfer_coins_to_output(50, token_asset_id.clone(), address.clone())
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // Check the balance of the contract of its own asset
    let result = token_instance
        .get_balance(target.clone(), token_asset_id.clone())
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 10000 - 50);

    // Inspect the wallet for alt tokens
    let token_asset_id_array: [u8; 32] = token_contract_id.into();
    let coins = wallet
        .get_spendable_coins(&AssetId::from(token_asset_id_array), 50)
        .await
        .unwrap();
    assert_eq!(coins[0].amount, 50u64.into());

    // Deposit some native assets
    swayswap_instance
        .deposit()
        .call_params(CallParameters::new(Some(50), None))
        .call()
        .await
        .unwrap();

    // deposit some alt tokens into the SwaySwap contract
    swayswap_instance
        .deposit()
        .call_params(CallParameters::new(
            Some(50),
            Some(AssetId::from(token_asset_id_array)),
        ))
        .call()
        .await
        .unwrap();

    // Add some initial liquidity
    swayswap_instance
        .add_liquidity(1, 50, 1000)
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // Inspect the wallet for LP tokens
    let swayswap_asset_id_array: [u8; 32] = swayswap_contract_id.into();
    assert_eq!(
        wallet
            .get_spendable_coins(&AssetId::from(swayswap_asset_id_array), 1)
            .await
            .unwrap()[0]
            .amount,
        50u64.into()
    );

    // Fund the wallet again with some alt tokens
    token_instance
        .transfer_coins_to_output(100, token_asset_id.clone(), address.clone())
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // Deposit some native assets
    swayswap_instance
        .deposit()
        .call_params(CallParameters::new(Some(100), None))
        .call()
        .await
        .unwrap();

    // Deposit some alt tokens
    swayswap_instance
        .deposit()
        .call_params(CallParameters::new(
            Some(100),
            Some(AssetId::from(token_asset_id_array)),
        ))
        .call()
        .await
        .unwrap();

    // Add liquidity for the second time.
    let result = swayswap_instance
        .add_liquidity(1, 100, 1000)
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, 33);

    // Inspect the wallet for LP tokens - should see 50 LP tokens + 33 LP tokens
    let lp_tokens = wallet
        .get_spendable_coins(&AssetId::from(swayswap_asset_id_array), 83)
        .await
        .unwrap();
    assert!(
        (lp_tokens[0].amount == 33u64.into()) && (lp_tokens[1].amount == 50u64.into())
            || (lp_tokens[0].amount == 50u64.into()) && (lp_tokens[1].amount == 33u64.into())
    );

    ///////////////////
    // Remove liquidity
    ///////////////////
    let result = swayswap_instance
        .remove_liquidity(30, 30, 1000)
        .call_params(CallParameters::new(
            Some(40),
            Some(AssetId::from(swayswap_asset_id_array)),
        ))
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value.eth_amount, 72);
    assert_eq!(result.value.token_amount, 72);

    // Inspect the wallet for LP tokens - should see 43 LP tokens
    assert_eq!(
        wallet
            .get_spendable_coins(&AssetId::from(swayswap_asset_id_array), 43)
            .await
            .unwrap()[0]
            .amount,
        43u64.into()
    );

    // Inspect the wallet for alt tokens - should see 72 ~= 150 * 40/83
    assert_eq!(
        wallet
            .get_spendable_coins(&AssetId::from(token_asset_id_array), 72)
            .await
            .unwrap()[0]
            .amount,
        72u64.into()
    ); // ~= 150 * 40/83

    ////////////////////
    // SWAP WITH MINIMUM
    ////////////////////
    // ETH -> TOKEN
    swayswap_instance
        .swap_with_minimum(10, 1000)
        .call_params(CallParameters::new(Some(40), None))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // TOKEN -> ETH
    swayswap_instance
        .swap_with_minimum(10, 1000)
        .call_params(CallParameters::new(
            Some(40),
            Some(AssetId::from(token_asset_id_array)),
        ))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
}
