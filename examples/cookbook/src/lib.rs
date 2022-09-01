#[cfg(test)]
mod tests {
    use fuels::prelude::Error;

    #[tokio::test]
    async fn liquidity() -> Result<(), Error> {
        use fuels::prelude::*;
        use fuels::test_helpers::{AssetConfig, WalletsConfig};

        // ANCHOR: liquidity_abigen
        abigen!(
            MyContract,
            "packages/fuels/tests/test_projects/liquidity_pool/out/debug/liquidity_pool-flat-abi.json"
        );
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
        let wallets = launch_custom_provider_and_get_wallets(wallet_config, None).await;
        let wallet = &wallets[0];
        // ANCHOR_END: liquidity_wallet

        // ANCHOR: liquidity_deploy
        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/test_projects/liquidity_pool/out/debug/liquidity_pool.bin",
            wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        let contract_instance =
            MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();
        // ANCHOR_END: liquidity_deploy

        // ANCHOR: liquidity_deposit
        let deposit_amount = 1_000_000;
        let call_params = CallParameters::new(Some(deposit_amount), Some(base_asset_id), None);
        contract_instance
            .deposit(wallet.address().into())
            .call_params(call_params)
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: liquidity_deposit

        // ANCHOR: liquidity_withdraw
        let lp_asset_id = AssetId::from(*contract_id.hash());
        let lp_token_balance = wallet.get_asset_balance(&lp_asset_id).await?;

        let call_params = CallParameters::new(Some(lp_token_balance), Some(lp_asset_id), None);
        contract_instance
            .withdraw(wallet.address().into())
            .call_params(call_params)
            .append_variable_outputs(1)
            .call()
            .await?;

        let base_balance = wallet.get_asset_balance(&base_asset_id).await?;
        assert_eq!(base_balance, deposit_amount);
        // ANCHOR_END: liquidity_withdraw

        Ok(())
    }

    #[tokio::test]
    async fn custom_chain() -> Result<(), Error> {
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
        let (client, _) =
            setup_test_client(coins, Some(node_config), Some(consensus_parameters_config)).await;
        let _provider = Provider::new(client);
        // ANCHOR_END: custom_chain_client
        Ok(())
    }

    #[tokio::test]
    async fn transfer_multiple() -> Result<(), Error> {
        // ANCHOR: transfer_multiple
        use fuels::prelude::*;
        use std::str::FromStr;

        // ANCHOR: transfer_multiple_setup
        let mut wallet_1 = WalletUnlocked::new_random(None);
        let mut wallet_2 = WalletUnlocked::new_random(None);

        const NUM_ASSETS: u64 = 5;
        const AMOUNT: u64 = 100_000;
        const NUM_COINS: u64 = 10;
        let (coins, _) =
            setup_multiple_assets_coins(wallet_1.address(), NUM_ASSETS, NUM_COINS, AMOUNT);

        let (provider, _) = setup_test_provider(coins, None).await;

        wallet_1.set_provider(provider.clone());
        wallet_2.set_provider(provider.clone());
        // ANCHOR_END: transfer_multiple_setup

        // ANCHOR: transfer_multiple_inout
        let balances = wallet_1.get_balances().await?;

        let mut inputs = vec![];
        let mut outputs = vec![];
        for (id_string, amount) in balances {
            let id = AssetId::from_str(&id_string).unwrap();

            let input = wallet_1.get_asset_inputs_for_amount(id, amount, 0).await?;
            inputs.extend(input);

            let output = wallet_1.get_asset_outputs_for_amount(wallet_2.address(), id, amount);
            outputs.extend(output);
        }
        // ANCHOR_END: transfer_multiple_inout

        // ANCHOR: transfer_multiple_transaction
        let mut tx = provider.build_transfer_tx(&inputs, &outputs, TxParameters::default());
        wallet_1.sign_transaction(&mut tx).await?;

        let _receipts = provider.send_transaction(&tx).await?;

        let balances = wallet_1.get_balances().await?;
        assert!(balances.is_empty());
        // ANCHOR_END: transfer_multiple_transaction

        // ANCHOR_END: transfer_multiple
        Ok(())
    }
}
