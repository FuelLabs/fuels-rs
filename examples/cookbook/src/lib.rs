#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fuels::{
        accounts::{predicate::Predicate, wallet::WalletUnlocked, Account, ViewOnlyAccount},
        core::constants::BASE_ASSET_ID,
        prelude::Result,
        test_helpers::{setup_single_asset_coins, setup_test_provider},
        types::{
            bech32::Bech32Address,
            transaction::TxPolicies,
            transaction_builders::{
                BuildableTransaction, ScriptTransactionBuilder, TransactionBuilder,
            },
            tx_status::TxStatus,
            AssetId,
        },
    };

    #[tokio::test]
    async fn liquidity() -> Result<()> {
        use fuels::{
            prelude::*,
            test_helpers::{AssetConfig, WalletsConfig},
            types::Bits256,
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
        let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;
        let wallet = &wallets[0];
        // ANCHOR_END: liquidity_wallet

        // ANCHOR: liquidity_deploy
        let contract_id = Contract::load_from(
            "../../packages/fuels/tests/contracts/liquidity_pool/out/debug/liquidity_pool.bin",
            LoadConfiguration::default(),
        )?
        .deploy(wallet, TxPolicies::default())
        .await?;

        let contract_methods = MyContract::new(contract_id.clone(), wallet.clone()).methods();
        // ANCHOR_END: liquidity_deploy

        // ANCHOR: liquidity_deposit
        let deposit_amount = 1_000_000;
        let call_params = CallParameters::default()
            .with_amount(deposit_amount)
            .with_asset_id(base_asset_id);

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
            .with_amount(lp_token_balance)
            .with_asset_id(lp_asset_id);

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
        use fuels::{
            prelude::*,
            tx::{ConsensusParameters, FeeParameters, TxParameters},
        };
        // ANCHOR_END: custom_chain_import

        // ANCHOR: custom_chain_consensus
        let tx_params = TxParameters::default()
            .with_max_gas_per_tx(1_000)
            .with_max_inputs(2);
        let fee_params = FeeParameters::default().with_gas_price_factor(10);

        let consensus_parameters = ConsensusParameters {
            tx_params,
            fee_params,
            ..Default::default()
        };

        let chain_config = ChainConfig {
            consensus_parameters,
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
        let node_config = Config::default();
        let _provider =
            setup_test_provider(coins, vec![], Some(node_config), Some(chain_config)).await?;
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

        let provider = setup_test_provider(coins, vec![], None, None).await?;

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

            let input = wallet_1.get_asset_inputs_for_amount(id, amount).await?;
            inputs.extend(input);

            let output = wallet_1.get_asset_outputs_for_amount(wallet_2.address(), id, amount);
            outputs.extend(output);
        }
        // ANCHOR_END: transfer_multiple_inout

        // ANCHOR: transfer_multiple_transaction
        let mut tb =
            ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
        tb.add_signer(wallet_1.clone())?;

        let tx = tb.build(&provider).await?;

        provider.send_transaction_and_await_commit(tx).await?;

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
            database_type: DbType::RocksDb(Some(PathBuf::from("/tmp/.spider/db"))),
            ..Config::default()
        };
        // ANCHOR_END: create_or_use_rocksdb

        launch_custom_provider_and_get_wallets(Default::default(), Some(provider_config), None)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn custom_transaction() -> Result<()> {
        let mut hot_wallet = WalletUnlocked::new_random(None);
        let mut cold_wallet = WalletUnlocked::new_random(None);

        let code_path = "../../packages/fuels/tests/predicates/swap/out/debug/swap.bin";
        let mut predicate = Predicate::load_from(code_path)?;

        let num_coins = 5;
        let amount = 1000;
        let bridged_asset_id = AssetId::from([1u8; 32]);
        let base_coins =
            setup_single_asset_coins(hot_wallet.address(), BASE_ASSET_ID, num_coins, amount);
        let other_coins =
            setup_single_asset_coins(predicate.address(), bridged_asset_id, num_coins, amount);

        let provider = setup_test_provider(
            base_coins.into_iter().chain(other_coins).collect(),
            vec![],
            None,
            None,
        )
        .await?;

        hot_wallet.set_provider(provider.clone());
        cold_wallet.set_provider(provider.clone());
        predicate.set_provider(provider.clone());

        // ANCHOR: custom_tx_receiver
        let ask_amount = 100;
        let locked_amount = 500;
        let bridged_asset_id = AssetId::from([1u8; 32]);
        let receiver = Bech32Address::from_str(
            "fuel1p8qt95dysmzrn2rmewntg6n6rg3l8ztueqafg5s6jmd9cgautrdslwdqdw",
        )?;
        // ANCHOR_END: custom_tx_receiver

        // ANCHOR: custom_tx
        let tb = ScriptTransactionBuilder::default();
        // ANCHOR_END: custom_tx

        // ANCHOR: custom_tx_io_base
        let base_inputs = hot_wallet
            .get_asset_inputs_for_amount(BASE_ASSET_ID, ask_amount)
            .await?;
        let base_outputs =
            hot_wallet.get_asset_outputs_for_amount(&receiver, BASE_ASSET_ID, ask_amount);
        // ANCHOR_END: custom_tx_io_base

        // ANCHOR: custom_tx_io_other
        let other_asset_inputs = predicate
            .get_asset_inputs_for_amount(bridged_asset_id, locked_amount)
            .await?;
        let other_asset_outputs =
            predicate.get_asset_outputs_for_amount(cold_wallet.address(), bridged_asset_id, 500);
        // ANCHOR_END: custom_tx_io_other

        // ANCHOR: custom_tx_io
        let inputs = base_inputs
            .into_iter()
            .chain(other_asset_inputs.into_iter())
            .collect();
        let outputs = base_outputs
            .into_iter()
            .chain(other_asset_outputs.into_iter())
            .collect();

        let mut tb = tb.with_inputs(inputs).with_outputs(outputs);
        // ANCHOR_END: custom_tx_io

        // ANCHOR: custom_tx_add_signer
        tb.add_signer(hot_wallet.clone())?;
        // ANCHOR_END: custom_tx_add_signer

        // ANCHOR: custom_tx_adjust
        hot_wallet.adjust_for_fee(&mut tb, 100).await?;
        // ANCHOR_END: custom_tx_adjust

        // ANCHOR: custom_tx_policies
        let tx_policies = TxPolicies::default().with_gas_price(1);
        let tb = tb.with_tx_policies(tx_policies);
        // ANCHOR_END: custom_tx_policies

        // ANCHOR: custom_tx_build
        let tx = tb.build(&provider).await?;
        let tx_id = provider.send_transaction(tx).await?;
        // ANCHOR_END: custom_tx_build

        // ANCHOR: custom_tx_verify
        let status = provider.tx_status(&tx_id).await?;
        assert!(matches!(status, TxStatus::Success { .. }));

        let balance = cold_wallet.get_asset_balance(&bridged_asset_id).await?;
        assert_eq!(balance, locked_amount);
        // ANCHOR_END: custom_tx_verify

        Ok(())
    }
}
