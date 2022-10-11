#[cfg(test)]
mod tests {
    use fuels::prelude::Error;

    #[tokio::test]
    async fn create_random_wallet() {
        // ANCHOR: create_random_wallet
        use fuels::prelude::*;

        // Use the test helper to setup a test provider.
        let (provider, _address) = setup_test_provider(vec![], vec![], None).await;

        // Create the wallet.
        let _wallet = WalletUnlocked::new_random(Some(provider));
        // ANCHOR_END: create_random_wallet
    }

    #[tokio::test]
    async fn create_wallet_from_secret_key() -> Result<(), Box<dyn std::error::Error>> {
        // ANCHOR: create_wallet_from_secret_key
        use fuels::prelude::*;
        use fuels::signers::fuel_crypto::SecretKey;
        use std::str::FromStr;

        // Use the test helper to setup a test provider.
        let (provider, _address) = setup_test_provider(vec![], vec![], None).await;

        // Setup the private key.
        let secret = SecretKey::from_str(
            "5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1",
        )?;

        // Create the wallet.
        let _wallet = WalletUnlocked::new_from_private_key(secret, Some(provider));
        // ANCHOR_END: create_wallet_from_secret_key
        Ok(())
    }

    #[tokio::test]
    async fn create_wallet_from_mnemonic() -> Result<(), Error> {
        // ANCHOR: create_wallet_from_mnemonic
        use fuels::prelude::*;

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Use the test helper to setup a test provider.
        let (provider, _address) = setup_test_provider(vec![], vec![], None).await;

        // Create first account from mnemonic phrase.
        let _wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            phrase,
            Some(provider.clone()),
            "m/44'/1179993420'/0'/0/0",
        )?;

        // Or with the default derivation path
        let wallet = WalletUnlocked::new_from_mnemonic_phrase(phrase, Some(provider))?;

        let expected_address = "fuel17x9kg3k7hqf42396vqenukm4yf59e5k0vj4yunr4mae9zjv9pdjszy098t";

        assert_eq!(wallet.address().to_string(), expected_address);
        // ANCHOR_END: create_wallet_from_mnemonic
        Ok(())
    }

    #[tokio::test]
    async fn create_and_restore_json_wallet() -> Result<(), Error> {
        // ANCHOR: create_and_restore_json_wallet
        use fuels::prelude::*;

        let dir = std::env::temp_dir();
        let mut rng = rand::thread_rng();

        // Use the test helper to setup a test provider.
        let (provider, _address) = setup_test_provider(vec![], vec![], None).await;

        let password = "my_master_password";

        // Create a wallet to be stored in the keystore.
        let (_wallet, uuid) =
            WalletUnlocked::new_from_keystore(&dir, &mut rng, password, Some(provider.clone()))?;

        let path = dir.join(uuid);

        let _recovered_wallet = WalletUnlocked::load_keystore(&path, password, Some(provider))?;
        // ANCHOR_END: create_and_restore_json_wallet
        Ok(())
    }

    #[tokio::test]
    async fn create_and_store_mnemonic_wallet() -> Result<(), Error> {
        // ANCHOR: create_and_store_mnemonic_wallet
        use fuels::prelude::*;

        let dir = std::env::temp_dir();

        let phrase =
            "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

        // Use the test helper to setup a test provider.
        let (provider, _address) = setup_test_provider(vec![], vec![], None).await;

        // Create first account from mnemonic phrase.
        let wallet = WalletUnlocked::new_from_mnemonic_phrase(phrase, Some(provider))?;

        let password = "my_master_password";

        // Encrypts and stores it on disk. Can be recovered using `Wallet::load_keystore`.
        let _uuid = wallet.encrypt(&dir, password)?;
        // ANCHOR_END: create_and_store_mnemonic_wallet
        Ok(())
    }

    #[tokio::test]
    async fn wallet_transfer() -> Result<(), Error> {
        // ANCHOR: wallet_transfer
        use fuels::prelude::*;

        // Setup 2 test wallets with 1 coin each
        let num_wallets = Some(2);
        let coins_per_wallet = Some(1);
        let coin_amount = Some(1);

        let wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(num_wallets, coins_per_wallet, coin_amount),
            None,
        )
        .await;

        // Transfer the base asset with amount 1 from wallet 1 to wallet 2
        let asset_id = Default::default();
        let _receipts = wallets[0]
            .transfer(wallets[1].address(), 1, asset_id, TxParameters::default())
            .await?;

        let wallet_2_final_coins = wallets[1].get_coins(BASE_ASSET_ID).await?;

        // Check that wallet 2 now has 2 coins
        assert_eq!(wallet_2_final_coins.len(), 2);

        // ANCHOR_END: wallet_transfer
        Ok(())
    }

    #[tokio::test]
    async fn wallet_contract_transfer() -> Result<(), Error> {
        use fuels::prelude::*;
        use rand::Fill;

        let mut rng = rand::thread_rng();

        let base_asset = AssetConfig {
            id: BASE_ASSET_ID,
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
        let wallet = launch_custom_provider_and_get_wallets(wallet_config, None)
            .await
            .pop()
            .unwrap();

        let contract_id = Contract::deploy(
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
            StorageConfiguration::default(),
        )
        .await?;

        // ANCHOR: wallet_contract_transfer
        // Check the current balance of the contract with id 'contract_id'
        let contract_balances = wallet
            .get_provider()?
            .get_contract_balances(&contract_id)
            .await?;
        assert!(contract_balances.is_empty());

        // Transfer an amount of 300 to the contract
        let amount = 300;
        let asset_id = random_asset_id;
        let _receipts = wallet
            .force_transfer_to_contract(&contract_id, amount, asset_id, TxParameters::default())
            .await?;

        // Check that the contract now has 1 coin
        let contract_balances = wallet
            .get_provider()?
            .get_contract_balances(&contract_id)
            .await?;
        assert_eq!(contract_balances.len(), 1);

        let random_asset_id_key = format!("{:#x}", random_asset_id);
        let random_asset_balance = contract_balances.get(&random_asset_id_key).unwrap();
        assert_eq!(*random_asset_balance, 300);
        // ANCHOR_END: wallet_contract_transfer

        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_multiple_wallets() -> Result<(), Error> {
        // ANCHOR: multiple_wallets_helper
        use fuels::prelude::*;
        // This helper will launch a local node and provide 10 test wallets linked to it.
        // The initial balance defaults to 1 coin per wallet with an amount of 1_000_000_000
        let wallets = launch_custom_provider_and_get_wallets(WalletsConfig::default(), None).await;
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
        // Launches a local node and provides test wallets as specified by the config
        let wallets = launch_custom_provider_and_get_wallets(config, None).await;
        // ANCHOR_END: setup_5_wallets
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_wallet_multiple_assets() -> Result<(), Error> {
        // ANCHOR: multiple_assets_wallet
        // ANCHOR: multiple_assets_coins
        use fuels::prelude::*;
        let mut wallet = WalletUnlocked::new_random(None);
        let num_assets = 5; // 5 different assets
        let coins_per_asset = 10; // Per asset id, 10 coins in the wallet
        let amount_per_coin = 15; // For each coin (UTXO) of the asset, amount of 15

        let (coins, asset_ids) = setup_multiple_assets_coins(
            wallet.address(),
            num_assets,
            coins_per_asset,
            amount_per_coin,
        );
        // ANCHOR_END: multiple_assets_coins
        let (provider, _socket_addr) = setup_test_provider(coins.clone(), vec![], None).await;
        wallet.set_provider(provider);
        // ANCHOR_END: multiple_assets_wallet
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn setup_wallet_custom_assets() -> Result<(), rand::Error> {
        // ANCHOR: custom_assets_wallet
        use fuels::prelude::*;
        use rand::Fill;

        let mut wallet = WalletUnlocked::new_random(None);
        let mut rng = rand::thread_rng();

        let asset_base = AssetConfig {
            id: BASE_ASSET_ID,
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

        let coins = setup_custom_assets_coins(wallet.address(), &assets);
        let (provider, _socket_addr) = setup_test_provider(coins, vec![], None).await;
        wallet.set_provider(provider);
        // ANCHOR_END: custom_assets_wallet
        // ANCHOR: custom_assets_wallet_short
        let num_wallets = 1;
        let wallet_config = WalletsConfig::new_multiple_assets(num_wallets, assets);
        let wallets = launch_custom_provider_and_get_wallets(wallet_config, None).await;
        // ANCHOR_END: custom_assets_wallet_short
        Ok(())
    }

    #[tokio::test]
    #[allow(unused_variables)]
    async fn get_balances() -> Result<(), Error> {
        use fuels::prelude::{
            launch_provider_and_get_wallet, BASE_ASSET_ID, DEFAULT_COIN_AMOUNT, DEFAULT_NUM_COINS,
        };
        use fuels::tx::AssetId;
        use std::collections::HashMap;

        let wallet = launch_provider_and_get_wallet().await;
        // ANCHOR: get_asset_balance
        let asset_id: AssetId = BASE_ASSET_ID;
        let balance: u64 = wallet.get_asset_balance(&asset_id).await?;
        // ANCHOR_END: get_asset_balance
        // ANCHOR: get_balances
        let balances: HashMap<String, u64> = wallet.get_balances().await?;
        // ANCHOR_END: get_balances

        // ANCHOR: get_balance_hashmap
        let asset_id_key = format!("{:#x}", asset_id);
        let asset_balance = balances.get(&asset_id_key).unwrap();
        // ANCHOR_END: get_balance_hashmap

        assert_eq!(*asset_balance, DEFAULT_COIN_AMOUNT * DEFAULT_NUM_COINS);

        Ok(())
    }
}
