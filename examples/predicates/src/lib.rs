#[cfg(test)]
mod tests {
    use fuels::{
        accounts::{predicate::Predicate, Account},
        crypto::{Message, SecretKey},
        prelude::*,
        types::B512,
    };

    #[tokio::test]
    async fn predicate_example() -> Result<()> {
        // ANCHOR: predicate_wallets
        let secret_key1: SecretKey =
            "0x862512a2363db2b3a375c0d4bbbd27172180d89f23f2e259bac850ab02619301".parse()?;

        let secret_key2: SecretKey =
            "0x37fa81c84ccd547c30c176b118d5cb892bdb113e8e80141f266519422ef9eefd".parse()?;

        let secret_key3: SecretKey =
            "0x976e5c3fa620092c718d852ca703b6da9e3075b9f2ecb8ed42d9f746bf26aafb".parse()?;

        let mut wallet = WalletUnlocked::new_from_private_key(secret_key1, None);
        let mut wallet2 = WalletUnlocked::new_from_private_key(secret_key2, None);
        let mut wallet3 = WalletUnlocked::new_from_private_key(secret_key3, None);
        let mut receiver = WalletUnlocked::new_random(None);
        // ANCHOR_END: predicate_wallets

        // ANCHOR: predicate_coins
        let asset_id = AssetId::default();
        let num_coins = 32;
        let amount = 64;
        let initial_balance = amount * num_coins;
        let all_coins = [&wallet, &wallet2, &wallet3, &receiver]
            .iter()
            .flat_map(|wallet| {
                setup_single_asset_coins(wallet.address(), asset_id, num_coins, amount)
            })
            .collect::<Vec<_>>();

        let provider = setup_test_provider(all_coins, vec![], None, None).await?;

        [&mut wallet, &mut wallet2, &mut wallet3, &mut receiver]
            .iter_mut()
            .for_each(|wallet| {
                wallet.set_provider(provider.clone());
            });
        // ANCHOR_END: predicate_coins

        let data_to_sign = Message::new([0; 32]);
        let signature1: B512 = wallet.sign(data_to_sign).await?.as_ref().try_into()?;
        let signature2: B512 = wallet2.sign(data_to_sign).await?.as_ref().try_into()?;
        let signature3: B512 = wallet3.sign(data_to_sign).await?.as_ref().try_into()?;

        let signatures = [signature1, signature2, signature3];

        // ANCHOR: predicate_load
        abigen!(Predicate(
            name = "MyPredicate",
            abi = "packages/fuels/tests/predicates/signatures/out/debug/signatures-abi.json"
        ));

        let predicate_data = MyPredicateEncoder::default().encode_data(signatures)?;
        let code_path = "../../packages/fuels/tests/predicates/signatures/out/debug/signatures.bin";

        let predicate: Predicate = Predicate::load_from(code_path)?
            .with_provider(provider)
            .with_data(predicate_data);
        // ANCHOR_END: predicate_load

        // ANCHOR: predicate_receive
        let amount_to_predicate = 512;

        wallet
            .transfer(
                predicate.address(),
                amount_to_predicate,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        let predicate_balance = predicate.get_asset_balance(&asset_id).await?;
        assert_eq!(predicate_balance, amount_to_predicate);
        // ANCHOR_END: predicate_receive

        // ANCHOR: predicate_spend
        predicate
            .transfer(
                receiver.address(),
                amount_to_predicate,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        let receiver_balance_after = receiver.get_asset_balance(&asset_id).await?;
        assert_eq!(
            initial_balance + amount_to_predicate,
            receiver_balance_after
        );

        let predicate_balance = predicate.get_asset_balance(&asset_id).await?;
        assert_eq!(predicate_balance, 0);
        // ANCHOR_END: predicate_spend

        Ok(())
    }

    #[tokio::test]
    async fn predicate_data_example() -> Result<()> {
        // ANCHOR: predicate_data_setup
        let asset_id = AssetId::default();
        let wallets_config = WalletsConfig::new_multiple_assets(
            2,
            vec![AssetConfig {
                id: asset_id,
                num_coins: 1,
                coin_amount: 1_000,
            }],
        );

        let wallets = &launch_custom_provider_and_get_wallets(wallets_config, None, None).await?;

        let first_wallet = &wallets[0];
        let second_wallet = &wallets[1];

        abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/basic_predicate/out/debug/basic_predicate-abi.json"));
        // ANCHOR_END: predicate_data_setup

        // ANCHOR: with_predicate_data
        let predicate_data = MyPredicateEncoder::default().encode_data(4096, 4096)?;
        let code_path =
            "../../packages/fuels/tests/predicates/basic_predicate/out/debug/basic_predicate.bin";

        let predicate: Predicate = Predicate::load_from(code_path)?
            .with_provider(first_wallet.try_provider()?.clone())
            .with_data(predicate_data);
        // ANCHOR_END: with_predicate_data

        // ANCHOR: predicate_data_lock_amount
        // First wallet transfers amount to predicate.
        first_wallet
            .transfer(predicate.address(), 500, asset_id, TxPolicies::default())
            .await?;

        // Check predicate balance.
        let balance = predicate.get_asset_balance(&AssetId::default()).await?;

        assert_eq!(balance, 500);
        // ANCHOR_END: predicate_data_lock_amount

        // ANCHOR: predicate_data_unlock
        let amount_to_unlock = 500;

        predicate
            .transfer(
                second_wallet.address(),
                amount_to_unlock,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        // Predicate balance is zero.
        let balance = predicate.get_asset_balance(&AssetId::default()).await?;

        assert_eq!(balance, 0);

        // Second wallet balance is updated.
        let balance = second_wallet.get_asset_balance(&AssetId::default()).await?;
        assert_eq!(balance, 1500);
        // ANCHOR_END: predicate_data_unlock
        Ok(())
    }
}
