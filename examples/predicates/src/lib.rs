#[cfg(test)]
mod tests {
    use fuels::{
        accounts::{Account, predicate::Predicate, signers::private_key::PrivateKeySigner},
        crypto::Message,
        prelude::*,
        types::B512,
    };
    use rand::thread_rng;

    #[tokio::test]
    async fn predicate_example() -> Result<()> {
        // ANCHOR: predicate_signers
        let wallet_signer = PrivateKeySigner::new(
            "0x862512a2363db2b3a375c0d4bbbd27172180d89f23f2e259bac850ab02619301".parse()?,
        );
        let wallet2_signer = PrivateKeySigner::new(
            "0x37fa81c84ccd547c30c176b118d5cb892bdb113e8e80141f266519422ef9eefd".parse()?,
        );
        let wallet3_signer = PrivateKeySigner::new(
            "0x976e5c3fa620092c718d852ca703b6da9e3075b9f2ecb8ed42d9f746bf26aafb".parse()?,
        );
        let receiver_signer = PrivateKeySigner::random(&mut thread_rng());
        // ANCHOR_END: predicate_signers

        // ANCHOR: predicate_coins
        let asset_id = AssetId::zeroed();
        let num_coins = 32;
        let amount = 64;
        let initial_balance = amount * num_coins;
        let all_coins = [
            &wallet_signer,
            &wallet2_signer,
            &wallet3_signer,
            &receiver_signer,
        ]
        .iter()
        .flat_map(|signer| setup_single_asset_coins(signer.address(), asset_id, num_coins, amount))
        .collect::<Vec<_>>();

        let provider = setup_test_provider(all_coins, vec![], None, None).await?;

        let wallet = Wallet::new(wallet_signer, provider.clone());
        let wallet2 = Wallet::new(wallet2_signer, provider.clone());
        let wallet3 = Wallet::new(wallet3_signer, provider.clone());
        let receiver = Wallet::new(receiver_signer, provider.clone());
        // ANCHOR_END: predicate_coins

        let data_to_sign = Message::new([0; 32]);
        let signature1: B512 = wallet
            .signer()
            .sign(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;
        let signature2: B512 = wallet2
            .signer()
            .sign(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;
        let signature3: B512 = wallet3
            .signer()
            .sign(data_to_sign)
            .await?
            .as_ref()
            .try_into()?;

        let signatures = [signature1, signature2, signature3];

        // ANCHOR: predicate_load
        abigen!(Predicate(
            name = "MyPredicate",
            abi = "e2e/sway/predicates/signatures/out/release/signatures-abi.json"
        ));

        let predicate_data = MyPredicateEncoder::default().encode_data(signatures)?;
        let code_path = "../../e2e/sway/predicates/signatures/out/release/signatures.bin";

        let predicate: Predicate = Predicate::load_from(code_path)?
            .with_provider(provider)
            .with_data(predicate_data);
        // ANCHOR_END: predicate_load

        // ANCHOR: predicate_receive
        let amount_to_predicate = 500;

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
        let amount_to_receiver = 300;
        predicate
            .transfer(
                receiver.address(),
                amount_to_receiver,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        let receiver_balance_after = receiver.get_asset_balance(&asset_id).await?;
        assert_eq!(initial_balance + amount_to_receiver, receiver_balance_after);
        // ANCHOR_END: predicate_spend

        Ok(())
    }

    #[tokio::test]
    async fn predicate_data_example() -> Result<()> {
        // ANCHOR: predicate_data_setup
        let asset_id = AssetId::zeroed();
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

        abigen!(Predicate(
            name = "MyPredicate",
            abi = "e2e/sway/predicates/basic_predicate/out/release/basic_predicate-abi.json"
        ));
        // ANCHOR_END: predicate_data_setup

        // ANCHOR: with_predicate_data
        let predicate_data = MyPredicateEncoder::default().encode_data(4096, 4096)?;
        let code_path = "../../e2e/sway/predicates/basic_predicate/out/release/basic_predicate.bin";

        let predicate: Predicate = Predicate::load_from(code_path)?
            .with_provider(first_wallet.provider().clone())
            .with_data(predicate_data);
        // ANCHOR_END: with_predicate_data

        // ANCHOR: predicate_data_lock_amount
        // First wallet transfers amount to predicate.
        first_wallet
            .transfer(predicate.address(), 500, asset_id, TxPolicies::default())
            .await?;

        // Check predicate balance.
        let balance = predicate.get_asset_balance(&AssetId::zeroed()).await?;

        assert_eq!(balance, 500);
        // ANCHOR_END: predicate_data_lock_amount

        // ANCHOR: predicate_data_unlock
        let amount_to_unlock = 300;

        predicate
            .transfer(
                second_wallet.address(),
                amount_to_unlock,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        // Second wallet balance is updated.
        let balance = second_wallet.get_asset_balance(&AssetId::zeroed()).await?;
        assert_eq!(balance, 1300);
        // ANCHOR_END: predicate_data_unlock
        Ok(())
    }
}
