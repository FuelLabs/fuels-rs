#[cfg(test)]
mod tests {
    use fuels::{
        accounts::{
            keystore::Keystore,
            signers::{derivation::DEFAULT_DERIVATION_PATH, private_key::PrivateKeySigner},
        },
        crypto::SecretKey,
        prelude::*,
    };
    use rand::thread_rng;

    #[tokio::test]
    async fn create_random_wallet() -> Result<()> {
        // ANCHOR: create_random_wallet
        use fuels::prelude::*;

        // Use the test helper to setup a test provider.
        let provider = setup_test_provider(vec![], vec![], None, None).await?;

        // Create the wallet.
        let _wallet = Wallet::random(&mut thread_rng(), provider);
        // ANCHOR_END: create_random_wallet

        Ok(())
    }

    #[tokio::test]
    async fn create_wallet_from_secret_key() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        // ANCHOR: create_wallet_from_secret_key
        use std::str::FromStr;

        use fuels::{crypto::SecretKey, prelude::*};

        // Use the test helper to setup a test provider.
        let provider = setup_test_provider(vec![], vec![], None, None).await?;

        // Setup the private key.
        let secret = SecretKey::from_str(
            "5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1",
        )?;

        // Create the wallet.
        let _wallet = Wallet::new(PrivateKeySigner::new(secret), provider);
        // ANCHOR_END: create_wallet_from_secret_key
        Ok(())
    }

    #[tokio::test]
    async fn create_wallet_from_mnemonic() -> Result<()> {
        // ANCHOR: create_wallet_from_mnemonic
        use fuels::prelude::*;

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Use the test helper to setup a test provider.
        let provider = setup_test_provider(vec![], vec![], None, None).await?;

        // Create first account from mnemonic phrase.
        let key =
            SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/1179993420'/0'/0/0")?;
        let signer = PrivateKeySigner::new(key);
        let _wallet = Wallet::new(signer, provider.clone());

        // Or with the default derivation path.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, DEFAULT_DERIVATION_PATH)?;
        let signer = PrivateKeySigner::new(key);
        let wallet = Wallet::new(signer, provider);

        let expected_address = "fuel17x9kg3k7hqf42396vqenukm4yf59e5k0vj4yunr4mae9zjv9pdjszy098t";

        assert_eq!(wallet.address().to_string(), expected_address);
        // ANCHOR_END: create_wallet_from_mnemonic
        Ok(())
    }

    #[tokio::test]
    async fn create_and_store_mnemonic_key() -> Result<()> {
        // ANCHOR: create_and_store_mnemonic_key
        let dir = std::env::temp_dir();
        let keystore = Keystore::new(&dir);

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Create a key from the mnemonic phrase using the default derivation path.
        let key = SecretKey::new_from_mnemonic_phrase_with_path(phrase, DEFAULT_DERIVATION_PATH)?;
        let password = "my_master_password";

        // Encrypt and store the key on disk. It can be recovered using `Keystore::load_key`.
        let uuid = keystore.save_key(key, password, thread_rng())?;

        // Recover key from disk
        let recovered_key = keystore.load_key(&uuid, password)?;
        // ANCHOR_END: create_and_store_mnemonic_key

        assert_eq!(key, recovered_key);
        Ok(())
    }

    #[tokio::test]
    async fn wallet_transfer() -> Result<()> {
        // ANCHOR: wallet_transfer
        use fuels::prelude::*;

        // Setup 2 test wallets with 1 coin each.
        let num_wallets = 2;
        let coins_per_wallet = 1;
        let coin_amount = 2;

        let wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(num_wallets), Some(coins_per_wallet), Some(coin_amount)),
            None,
            None,
        )
        .await?;

        // Transfer the base asset with amount 1 from wallet 1 to wallet 2.
        let transfer_amount = 1;
        let asset_id = Default::default();
        let _res = wallets[0]
            .transfer(
                wallets[1].address(),
                transfer_amount,
                asset_id,
                TxPolicies::default(),
            )
            .await?;

        let wallet_2_final_coins = wallets[1].get_coins(AssetId::zeroed()).await?;

        // Check that wallet 2 now has 2 coins.
        assert_eq!(wallet_2_final_coins.len(), 2);
        // ANCHOR_END: wallet_transfer
        Ok(())
    }

    #[tokio::test]
    async fn wallet_contract_transfer() -> Result<()> {
        use fuels::prelude::*;
        use rand::Fill;

        let mut rng = rand::thread_rng();

        let base_asset = AssetConfig {
            id: AssetId::zeroed(),
            num_coins: 1,
            coin_amount: 1000,
        };

        let mut random_asset_id = AssetId::zeroed();
        random_asset_id.try_fill(&mut rng).unwrap();
        let random_asset = AssetConfig {
            id: random_asset_id,
            num_coins: 3,
            coin_amount: 100,
        };

        let wallet_config = WalletsConfig::new_multiple_assets(1, vec![random_asset, base_asset]);
        let wallet = launch_custom_provider_and_get_wallets(wallet_config, None, None)
            .await?
            .pop()
            .unwrap();

        let contract_id = Contract::load_from(
            "../../e2e/sway/contracts/contract_test/out/release/contract_test.bin",
            LoadConfiguration::default(),
        )?
        .deploy(&wallet, TxPolicies::default())
        .await?
        .contract_id;

        // ANCHOR: wallet_contract_transfer
        // Check the current balance of the contract with id 'contract_id'.
        let contract_balances = wallet
            .try_provider()?
            .get_contract_balances(&contract_id)
            .await?;
        assert!(contract_balances.is_empty());

        // Transfer an amount of 300 to the contract.
        let amount = 300;
        let asset_id = random_asset_id;
        let _res = wallet
            .force_transfer_to_contract(&contract_id, amount, asset_id, TxPolicies::default())
            .await?;

        // Check that the contract now has 1 coin.
        let contract_balances = wallet
            .try_provider()?
            .get_contract_balances(&contract_id)
            .await?;
        assert_eq!(contract_balances.len(), 1);

        let random_asset_balance = contract_balances.get(&random_asset_id).unwrap();
        assert_eq!(*random_asset_balance, 300);
        // ANCHOR_END: wallet_contract_transfer
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_multiple_wallets() -> Result<()> {
        // ANCHOR: multiple_wallets_helper
        use fuels::prelude::*;
        // This helper launches a local node and provides 10 test wallets linked to it.
        // The initial balance defaults to 1 coin per wallet with an amount of 1_000_000_000.
        let wallets =
            launch_custom_provider_and_get_wallets(WalletsConfig::default(), None, None).await?;
        // ANCHOR_END: multiple_wallets_helper

        // ANCHOR: setup_5_wallets
        let num_wallets = 5;
        let coins_per_wallet = 3;
        let amount_per_coin = 100;
        let config = WalletsConfig::new(
            Some(num_wallets),
            Some(coins_per_wallet),
            Some(amount_per_coin),
        );
        // Launches a local node and provides test wallets as specified by the config.
        let wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;
        // ANCHOR_END: setup_5_wallets
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_wallet_multiple_assets() -> Result<()> {
        // ANCHOR: multiple_assets_wallet
        // ANCHOR: multiple_assets_coins
        use fuels::prelude::*;
        let signer = PrivateKeySigner::random(&mut thread_rng());
        let num_assets = 5;
        let coins_per_asset = 10;
        let amount_per_coin = 15;

        let (coins, asset_ids) = setup_multiple_assets_coins(
            signer.address(),
            num_assets,
            coins_per_asset,
            amount_per_coin,
        );
        // ANCHOR_END: multiple_assets_coins
        let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
        let wallet = Wallet::new(signer, provider);
        // ANCHOR_END: multiple_assets_wallet
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_wallet_custom_assets() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // ANCHOR: custom_assets_wallet
        use fuels::prelude::*;
        use rand::Fill;

        let mut rng = rand::thread_rng();
        let signer = PrivateKeySigner::random(&mut rng);

        let asset_base = AssetConfig {
            id: AssetId::zeroed(),
            num_coins: 2,
            coin_amount: 4,
        };

        let mut asset_id_1 = AssetId::zeroed();
        asset_id_1.try_fill(&mut rng)?;
        let asset_1 = AssetConfig {
            id: asset_id_1,
            num_coins: 6,
            coin_amount: 8,
        };

        let mut asset_id_2 = AssetId::zeroed();
        asset_id_2.try_fill(&mut rng)?;
        let asset_2 = AssetConfig {
            id: asset_id_2,
            num_coins: 10,
            coin_amount: 12,
        };

        let assets = vec![asset_base, asset_1, asset_2];
        let coins = setup_custom_assets_coins(signer.address(), &assets);
        let provider = setup_test_provider(coins, vec![], None, None).await?;
        let wallet = Wallet::new(signer, provider.clone());
        // ANCHOR_END: custom_assets_wallet

        // ANCHOR: custom_assets_wallet_short
        let num_wallets = 1;
        let wallet_config = WalletsConfig::new_multiple_assets(num_wallets, assets);
        let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;
        // ANCHOR_END: custom_assets_wallet_short

        // ANCHOR: wallet_to_address
        let wallet = Wallet::random(&mut rng, provider);
        let address: Address = wallet.address().into();
        // ANCHOR_END: wallet_to_address
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn get_balances() -> Result<()> {
        use std::collections::HashMap;

        use fuels::{
            prelude::{DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS, launch_provider_and_get_wallet},
            types::AssetId,
        };

        let wallet = launch_provider_and_get_wallet().await?;
        // ANCHOR: get_asset_balance
        let asset_id = AssetId::zeroed();
        let balance: u64 = wallet.get_asset_balance(&asset_id).await?;
        // ANCHOR_END: get_asset_balance

        // ANCHOR: get_balances
        let balances: HashMap<String, u128> = wallet.get_balances().await?;
        // ANCHOR_END: get_balances

        // ANCHOR: get_balance_hashmap
        let asset_balance = balances.get(&asset_id.to_string()).unwrap();
        // ANCHOR_END: get_balance_hashmap

        assert_eq!(
            *asset_balance,
            (DEFAULT_COIN_AMOUNT * DEFAULT_NUM_COINS) as u128
        );

        Ok(())
    }

    #[tokio::test]
    async fn wallet_transfer_to_base_layer() -> Result<()> {
        // ANCHOR: wallet_withdraw_to_base
        use std::str::FromStr;

        use fuels::prelude::*;

        let wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(Some(1), None, None),
            None,
            None,
        )
        .await?;
        let wallet = wallets.first().unwrap();

        let amount = 1000;
        let base_layer_address = Address::from_str(
            "0x4710162c2e3a95a6faff05139150017c9e38e5e280432d546fae345d6ce6d8fe",
        )?;
        let base_layer_address = Bech32Address::from(base_layer_address);

        // Transfer an amount of 1000 to the specified base layer address.
        let response = wallet
            .withdraw_to_base_layer(&base_layer_address, amount, TxPolicies::default())
            .await?;

        let _block_height = wallet.provider().produce_blocks(1, None).await?;

        // Retrieve a message proof from the provider.
        let proof = wallet
            .try_provider()?
            .get_message_proof(&response.tx_id, &response.nonce, None, Some(2))
            .await?;

        // Verify the amount and recipient.
        assert_eq!(proof.amount, amount);
        assert_eq!(proof.recipient, base_layer_address);
        // ANCHOR_END: wallet_withdraw_to_base
        Ok(())
    }
}
