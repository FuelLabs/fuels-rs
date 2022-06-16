#[tokio::test]
// ANCHOR: create_random_wallet
async fn create_random_wallet() {
    use fuels::prelude::*;

    // Use the test helper to setup a test provider.
    let (provider, _address) = setup_test_provider(vec![], None).await;

    // Create the wallet.
    let _wallet = LocalWallet::new_random(Some(provider));
}
// ANCHOR_END: create_random_wallet

#[tokio::test]
// ANCHOR: create_wallet_from_secret_key
async fn create_wallet_from_secret_key() {
    use fuels::prelude::*;
    use fuels::signers::fuel_crypto::SecretKey;
    use std::str::FromStr;

    // Use the test helper to setup a test provider.
    let (provider, _address) = setup_test_provider(vec![], None).await;

    // Setup the private key.
    let secret =
        SecretKey::from_str("5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1")
            .unwrap();

    // Create the wallet.
    let _wallet = LocalWallet::new_from_private_key(secret, Some(provider));
}
// ANCHOR_END: create_wallet_from_secret_key

#[tokio::test]
// ANCHOR: create_wallet_from_mnemonic
async fn create_wallet_from_mnemonic() {
    use fuels::prelude::*;

    let phrase = "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

    // Use the test helper to setup a test provider.
    let (provider, _address) = setup_test_provider(vec![], None).await;

    // Create first account from mnemonic phrase.
    let _wallet = LocalWallet::new_from_mnemonic_phrase_with_path(
        phrase,
        Some(provider.clone()),
        "m/44'/1179993420'/0'/0/0",
    )
    .unwrap();

    // Or with the default derivation path
    let wallet = LocalWallet::new_from_mnemonic_phrase(phrase, Some(provider)).unwrap();

    let expected_address = "f18b6446deb8135544ba60333e5b7522685cd2cf64aa4e4c75df725149850b65";

    assert_eq!(wallet.address().to_string(), expected_address);
}
// ANCHOR_END: create_wallet_from_mnemonic

#[tokio::test]
// ANCHOR: create_and_restore_json_wallet
async fn create_and_restore_json_wallet() {
    use fuels::prelude::*;

    let dir = std::env::temp_dir();
    let mut rng = rand::thread_rng();

    // Use the test helper to setup a test provider.
    let (provider, _address) = setup_test_provider(vec![], None).await;

    let password = "my_master_password";

    // Create a wallet to be stored in the keystore.
    let (_wallet, uuid) =
        LocalWallet::new_from_keystore(&dir, &mut rng, password, Some(provider.clone())).unwrap();

    let path = dir.join(uuid);

    let _recovered_wallet = LocalWallet::load_keystore(&path, password, Some(provider)).unwrap();
}
// ANCHOR_END: create_and_restore_json_wallet

#[tokio::test]
// ANCHOR: create_and_store_mnemonic_wallet
async fn create_and_store_mnemonic_wallet() {
    use fuels::prelude::*;

    let dir = std::env::temp_dir();

    let phrase = "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

    // Use the test helper to setup a test provider.
    let (provider, _address) = setup_test_provider(vec![], None).await;

    // Create first account from mnemonic phrase.
    let wallet = LocalWallet::new_from_mnemonic_phrase(phrase, Some(provider)).unwrap();

    let password = "my_master_password";

    // Encrypts and stores it on disk. Can be recovered using `Wallet::load_keystore`.
    let _uuid = wallet.encrypt(&dir, password).unwrap();
}
// ANCHOR_END: create_and_store_mnemonic_wallet

#[tokio::test]
async fn wallet_transfer() -> Result<(), Box<dyn std::error::Error>> {
    use fuels::prelude::*;

    // Setup 2 test wallets with 1 coin each
    let wallets = launch_provider_and_get_wallets(WalletsConfig {
        num_wallets: 2,
        coins_per_wallet: 1,
        coin_amount: 1,
    })
    .await;

    // Transfer 1 from wallet 1 to wallet 2
    let asset_id = Default::default();
    let _receipts = wallets[0]
        .transfer(&wallets[1].address(), 1, asset_id, TxParameters::default())
        .await
        .unwrap();

    let wallet_2_final_coins = wallets[1].get_coins().await.unwrap();

    // Check that wallet 2 now has 2 coins
    assert_eq!(wallet_2_final_coins.len(), 2);
    Ok(())
}
