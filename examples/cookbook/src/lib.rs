#[cfg(test)]
mod tests {
    use fuels::prelude::Result;

    #[tokio::test]
    async fn liquidity() -> Result<()> {
        use fuels::{
            prelude::*,
            test_helpers::{AssetConfig, WalletsConfig},
        };

        // ANCHOR: liquidity_abigen
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/liquidity_pool/out/debug/liquidity_pool-abi.json"
        ));
        // ANCHOR_END: liquidity_abigen

        // ANCHOR: liquidity_wallet
        let base_asset_id: AssetId =
            "0x9ae5b658754e096e4d681c548daf46354495a437cc61492599e33fc64dcdc30c"
                .parse()
                .unwrap();

        let asset_ids = [AssetId::default(), base_asset_id];
        let asset_configs = asset_ids
            .map(|id| AssetConfig {
                id,
                num_coins: 1,
                coin_amount: 1_000_000,
            })
            .into();

        let wallet_config = WalletsConfig::new_multiple_assets(1, asset_configs);
        let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await;
        let wallet = &wallets[0];
        // ANCHOR_END: liquidity_wallet

        // ANCHOR: liquidity_deploy
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/liquidity_pool/out/debug/liquidity_pool.bin",
            wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();
        // ANCHOR_END: liquidity_deploy

        // ANCHOR: liquidity_deposit
        let deposit_amount = 1_000_000;
        let call_params = CallParameters::new(Some(deposit_amount), Some(base_asset_id), None);
        contract_methods
            .deposit(wallet.address().into())
            .call_params(call_params)?
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: liquidity_deposit

        // ANCHOR: liquidity_withdraw
        let lp_asset_id = AssetId::from(*contract_id.hash());
        let lp_token_balance = wallet.get_asset_balance(&lp_asset_id).await?;

        let call_params = CallParameters::new(Some(lp_token_balance), Some(lp_asset_id), None);
        contract_methods
            .withdraw(wallet.address().into())
            .call_params(call_params)?
            .append_variable_outputs(1)
            .call()
            .await?;

        let base_balance = wallet.get_asset_balance(&base_asset_id).await?;
        assert_eq!(base_balance, deposit_amount);
        // ANCHOR_END: liquidity_withdraw
        Ok(())
    }

    #[tokio::test]
    async fn custom_chain() -> Result<()> {
        use fuels::prelude::*;
        // ANCHOR: custom_chain_import
        use fuels::tx::ConsensusParameters;
        // ANCHOR_END: custom_chain_import

        // ANCHOR: custom_chain_consensus
        let consensus_parameters_config = ConsensusParameters::DEFAULT
            .with_max_gas_per_tx(1000)
            .with_gas_price_factor(10)
            .with_max_inputs(2);
        // ANCHOR_END: custom_chain_consensus

        // ANCHOR: custom_chain_coins
        let wallet = WalletUnlocked::new_random(None);
        let coins = setup_single_asset_coins(
            wallet.address(),
            Default::default(),
            DEFAULT_NUM_COINS,
            DEFAULT_COIN_AMOUNT,
        );
        // ANCHOR_END: custom_chain_coins

        // ANCHOR: custom_chain_client
        let node_config = Config::local_node();
        let (client, _) = setup_test_client(
            coins,
            vec![],
            Some(node_config),
            None,
            Some(consensus_parameters_config),
        )
        .await;
        let _provider = Provider::new(client);
        // ANCHOR_END: custom_chain_client
        Ok(())
    }

    #[tokio::test]
    async fn transfer_multiple() -> Result<()> {
        use std::str::FromStr;

        use fuels::prelude::*;

        // ANCHOR: transfer_multiple_setup
        let mut wallet_1 = WalletUnlocked::new_random(None);
        let mut wallet_2 = WalletUnlocked::new_random(None);

        const NUM_ASSETS: u64 = 5;
        const AMOUNT: u64 = 100_000;
        const NUM_COINS: u64 = 1;
        let (coins, _) =
            setup_multiple_assets_coins(wallet_1.address(), NUM_ASSETS, NUM_COINS, AMOUNT);

        let (provider, _) = setup_test_provider(coins, vec![], None, None).await;

        wallet_1.set_provider(provider.clone());
        wallet_2.set_provider(provider.clone());
        // ANCHOR_END: transfer_multiple_setup

        // ANCHOR: transfer_multiple_inout
        let balances = wallet_1.get_balances().await?;

        let mut inputs = vec![];
        let mut outputs = vec![];
        for (id_string, amount) in balances {
            let id = AssetId::from_str(&id_string).unwrap();

            // leave the base asset to cover transaction fees
            if id == BASE_ASSET_ID {
                continue;
            }
            let input = wallet_1.get_asset_inputs_for_amount(id, amount, 0).await?;
            inputs.extend(input);

            let output = wallet_1.get_asset_outputs_for_amount(wallet_2.address(), id, amount);
            outputs.extend(output);
        }
        // ANCHOR_END: transfer_multiple_inout

        // ANCHOR: transfer_multiple_transaction
        let mut tx = ScriptTransaction::new(inputs, outputs, TxParameters::default());
        wallet_1.sign_transaction(&mut tx).await?;

        let _receipts = provider.send_transaction(&tx).await?;

        let balances = wallet_2.get_balances().await?;

        assert_eq!(balances.len(), (NUM_ASSETS - 1) as usize);
        for (_, balance) in balances {
            assert_eq!(balance, AMOUNT);
        }
        // ANCHOR_END: transfer_multiple_transaction

        Ok(())
    }

    #[tokio::test]
    async fn modify_contract_call_transaction_inputs() -> Result<()> {
        // ANCHOR: modify_call_inputs_include
        use fuels::prelude::*;
        // ANCHOR_END: modify_call_inputs_include

        // ANCHOR: modify_call_inputs_setup
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json"
        ));

        let some_asset_id = AssetId::new([3; 32usize]);
        let coin_amount = 1_000_000;
        let asset_configs = [AssetId::default(), some_asset_id]
            .map(|id| AssetConfig {
                id,
                num_coins: 1,
                coin_amount,
            })
            .into();

        const NUM_WALLETS: u64 = 2;
        let wallet_config = WalletsConfig::new_multiple_assets(NUM_WALLETS, asset_configs);
        let mut wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await;

        let wallet_1 = wallets.pop().unwrap();
        let wallet_2 = wallets.pop().unwrap();
        // ANCHOR_END: modify_call_inputs_setup

        // ANCHOR: modify_call_inputs_instance
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet_1,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_instance = MyContract::new(contract_id, wallet_1.clone());

        let call_handler = contract_instance.methods().initialize_counter(42);
        let mut tx = call_handler.build_tx().await?;
        // ANCHOR_END: modify_call_inputs_instance

        // ANCHOR: modify_call_inputs_execute
        const SEND_AMOUNT: u64 = 1000;
        let input = wallet_1
            .get_asset_inputs_for_amount(some_asset_id, SEND_AMOUNT, 0)
            .await?;
        tx.inputs_mut().extend(input);

        let output =
            wallet_1.get_asset_outputs_for_amount(wallet_2.address(), some_asset_id, SEND_AMOUNT);
        tx.outputs_mut().extend(output);

        let provider = wallet_1.get_provider()?;
        let receipts = provider.send_transaction(&tx).await?;
        // ANCHOR_END: modify_call_inputs_execute

        // ANCHOR: modify_call_inputs_verify
        let response = call_handler.get_response(receipts)?;
        assert_eq!(response.value, 42);

        let balance_1 = wallet_1.get_asset_balance(&some_asset_id).await?;
        assert_eq!(balance_1, coin_amount - SEND_AMOUNT);

        let balance_2 = wallet_2.get_asset_balance(&some_asset_id).await?;
        assert_eq!(balance_2, coin_amount + SEND_AMOUNT);
        // ANCHOR_END: modify_call_inputs_verify

        Ok(())
    }
}
