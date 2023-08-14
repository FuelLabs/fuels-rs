#[cfg(test)]
mod tests {
    use fuels::types::Bits256;
    use fuels::{
        prelude::Result,
        types::transaction_builders::{ScriptTransactionBuilder, TransactionBuilder},
    };

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
        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/liquidity_pool/out/debug/liquidity_pool.bin",
            LoadConfiguration::default(),
        )?
        .deploy(wallet, TxParameters::default())
        .await?;

        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();
        // ANCHOR_END: liquidity_deploy

        // ANCHOR: liquidity_deposit
        let deposit_amount = 1_000_000;
        let call_params = CallParameters::default()
            .set_amount(deposit_amount)
            .set_asset_id(base_asset_id);

        contract_methods
            .deposit(wallet.address())
            .call_params(call_params)?
            .append_variable_outputs(1)
            .call()
            .await?;
        // ANCHOR_END: liquidity_deposit

        // ANCHOR: liquidity_withdraw
        let lp_asset_id = contract_id.asset_id(&Bits256::zeroed());
        let lp_token_balance = wallet.get_asset_balance(&lp_asset_id).await?;

        let call_params = CallParameters::default()
            .set_amount(lp_token_balance)
            .set_asset_id(lp_asset_id);

        contract_methods
            .withdraw(wallet.address())
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
        // ANCHOR: custom_chain_import
        use fuels::{fuel_node::ChainConfig, prelude::*, tx::ConsensusParameters};
        // ANCHOR_END: custom_chain_import

        // ANCHOR: custom_chain_consensus
        let consensus_parameters_config = ConsensusParameters::DEFAULT
            .with_max_gas_per_tx(1000)
            .with_gas_price_factor(10)
            .with_max_inputs(2);
        let chain_config = ChainConfig {
            transaction_parameters: consensus_parameters_config,
            ..ChainConfig::default()
        };
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

        // ANCHOR: custom_chain_provider
        let node_config = Config::local_node();
        let (_provider, _bound_address) =
            setup_test_provider(coins, vec![], Some(node_config), Some(chain_config)).await;
        // ANCHOR_END: custom_chain_provider
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

            let input = wallet_1
                .get_asset_inputs_for_amount(id, amount, None)
                .await?;
            inputs.extend(input);

            let output = wallet_1.get_asset_outputs_for_amount(wallet_2.address(), id, amount);
            outputs.extend(output);
        }
        // ANCHOR_END: transfer_multiple_inout

        // ANCHOR: transfer_multiple_transaction
        let mut tx =
            ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxParameters::default())
                .build()?;
        wallet_1.sign_transaction(&mut tx)?;

        provider.send_transaction(&tx).await?;

        let balances = wallet_2.get_balances().await?;

        assert_eq!(balances.len(), (NUM_ASSETS - 1) as usize);
        for (_, balance) in balances {
            assert_eq!(balance, AMOUNT);
        }
        // ANCHOR_END: transfer_multiple_transaction

        Ok(())
    }

    #[tokio::test]
    #[cfg(any(not(feature = "fuel-core-lib"), feature = "rocksdb"))]
    async fn create_or_use_rocksdb() -> Result<()> {
        use std::path::PathBuf;

        use fuels::prelude::*;

        // ANCHOR: create_or_use_rocksdb
        let provider_config = Config {
            database_path: PathBuf::from("/tmp/.spider/db"),
            database_type: DbType::RocksDb,
            ..Config::local_node()
        };
        // ANCHOR_END: create_or_use_rocksdb

        launch_custom_provider_and_get_wallets(Default::default(), Some(provider_config), None)
            .await;

        Ok(())
    }
}
