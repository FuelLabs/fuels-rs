#[cfg(test)]
mod tests {
    use fuels::{prelude::*, types::B512};

    #[tokio::test]
    async fn predicate_example() -> Result<(), Error> {
        use fuels::signers::fuel_crypto::SecretKey;

        // ANCHOR: predicate_wallets
        let secret_key1: SecretKey =
            "0x862512a2363db2b3a375c0d4bbbd27172180d89f23f2e259bac850ab02619301"
                .parse()
                .unwrap();

        let secret_key2: SecretKey =
            "0x37fa81c84ccd547c30c176b118d5cb892bdb113e8e80141f266519422ef9eefd"
                .parse()
                .unwrap();

        let secret_key3: SecretKey =
            "0x976e5c3fa620092c718d852ca703b6da9e3075b9f2ecb8ed42d9f746bf26aafb"
                .parse()
                .unwrap();

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

        let (provider, _) = setup_test_provider(all_coins, vec![], None, None).await;

        [&mut wallet, &mut wallet2, &mut wallet3, &mut receiver]
            .iter_mut()
            .for_each(|wallet| wallet.set_provider(provider.clone()));
        // ANCHOR_END: predicate_coins

        // ANCHOR: predicate_load
        abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_signatures/out/debug/predicate_signatures-abi.json"));

        let predicate = MyPredicate::load_from(
            "../../packages/fuels/tests/predicates/predicate_signatures/out/debug/predicate_signatures.bin",
        )?;
        // ANCHOR_END: predicate_load

        // ANCHOR: predicate_receive
        let amount_to_predicate = 512;

        predicate
            .receive(&wallet, amount_to_predicate, asset_id, None)
            .await?;

        let predicate_balance = provider
            .get_asset_balance(predicate.address(), asset_id)
            .await?;
        assert_eq!(predicate_balance, amount_to_predicate);
        // ANCHOR_END: predicate_receive

        // ANCHOR: predicate_signatures
        let data_to_sign = [0; 32];
        let signature1: B512 = wallet
            .sign_message(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;
        let signature2: B512 = wallet2
            .sign_message(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;
        let signature3: B512 = wallet3
            .sign_message(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;

        let signatures = [signature1, signature2, signature3];
        // ANCHOR_END: predicate_signatures

        // ANCHOR: predicate_spend
        predicate
            .encode_data(signatures)
            .spend(&receiver, amount_to_predicate, asset_id, None)
            .await?;

        let receiver_balance_after = provider
            .get_asset_balance(receiver.address(), asset_id)
            .await?;
        assert_eq!(
            initial_balance + amount_to_predicate,
            receiver_balance_after
        );

        let predicate_balance = provider
            .get_asset_balance(predicate.address(), asset_id)
            .await?;
        assert_eq!(predicate_balance, 0);
        // ANCHOR_END: predicate_spend

        Ok(())
    }

    #[tokio::test]
    async fn predicate_data_example() -> Result<(), Error> {
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

        let wallets = &launch_custom_provider_and_get_wallets(wallets_config, None, None).await;

        let first_wallet = &wallets[0];
        let second_wallet = &wallets[1];

        abigen!(Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_basic/out/debug/predicate_basic-abi.json"));

        let predicate = MyPredicate::load_from(
            "../../packages/fuels/tests/predicates/predicate_basic/out/debug/predicate_basic.bin",
        )?;
        // ANCHOR_END: predicate_data_setup

        // ANCHOR: predicate_data_lock_amount
        // First wallet transfers amount to predicate.
        predicate.receive(first_wallet, 500, asset_id, None).await?;

        // Check predicate balance.
        let balance = first_wallet
            .get_provider()?
            .get_asset_balance(predicate.address(), asset_id)
            .await?;

        assert_eq!(balance, 500);
        // ANCHOR_END: predicate_data_lock_amount
        //
        // ANCHOR: encode_predicate_data
        let predicate = predicate.encode_data(4096, 4096);
        // ANCHOR_END: encode_predicate_data

        // ANCHOR: predicate_data_unlock
        // We use the Predicate's `encode_data()` to encode the data we want to
        // send to the predicate. This is a builder pattern and the function
        // returns a new predicate.
        let amount_to_unlock = 500;

        predicate
            .encode_data(4096, 4096)
            .spend(second_wallet, amount_to_unlock, asset_id, None)
            .await?;

        // Predicate balance is zero.
        let balance = first_wallet
            .get_provider()?
            .get_asset_balance(predicate.address(), AssetId::default())
            .await?;

        assert_eq!(balance, 0);

        // Second wallet balance is updated.
        let balance = second_wallet.get_asset_balance(&AssetId::default()).await?;
        assert_eq!(balance, 1500);
        // ANCHOR_END: predicate_data_unlock
        Ok(())
    }
}
