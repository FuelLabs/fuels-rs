extern crate core;

pub mod provider;
pub mod wallet;

use std::error::Error;

use async_trait::async_trait;
#[doc(no_inline)]
pub use fuel_crypto;
use fuel_crypto::Signature;
use fuel_tx::{field, Cacheable, UniqueIdentifier};
use fuels_types::bech32::Bech32Address;
pub use wallet::{Wallet, WalletUnlocked};

/// Trait for signing transactions and messages
///
/// Implement this trait to support different signing modes, e.g. Ledger, hosted etc.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Signer: std::fmt::Debug + Send + Sync {
    type Error: Error + Send + Sync;
    /// Signs the hash of the provided message
    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error>;

    /// Signs the transaction
    async fn sign_transaction<Tx: Cacheable + UniqueIdentifier + field::Witnesses + Send>(
        &self,
        message: &mut Tx,
    ) -> Result<Signature, Self::Error>;

    /// Returns the signer's Fuel Address
    fn address(&self) -> &Bech32Address;
}

#[cfg(test)]
#[cfg(feature = "test-helpers")]
mod tests {
    use std::str::FromStr;

    use fuel_crypto::{Message, SecretKey};
    use fuel_tx::{
        field::Maturity, Address, AssetId, Bytes32, Chargeable, Input, Output, Transaction,
        TxPointer, UtxoId,
    };
    use fuels_core::{constants::BASE_ASSET_ID, parameters::TxParameters};
    use fuels_test_helpers::{setup_single_asset_coins, setup_test_client};
    use rand::{rngs::StdRng, RngCore, SeedableRng};

    use super::*;
    use crate::{provider::Provider, wallet::WalletUnlocked};

    #[tokio::test]
    async fn sign_and_verify() -> Result<(), Box<dyn Error>> {
        // ANCHOR: sign_message
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        rng.fill_bytes(&mut secret_seed);

        let secret = unsafe { SecretKey::from_bytes_unchecked(secret_seed) };

        // Create a wallet using the private key created above.
        let wallet = WalletUnlocked::new_from_private_key(secret, None);

        let message = "my message";

        let signature = wallet.sign_message(message).await?;

        // Check if signature is what we expect it to be
        assert_eq!(signature, Signature::from_str("0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d")?);

        // Recover address that signed the message
        let message = Message::new(message);
        let recovered_address = signature.recover(&message)?;

        assert_eq!(wallet.address().hash(), recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;
        Ok(())
        // ANCHOR_END: sign_message
    }

    #[tokio::test]
    async fn sign_tx_and_verify() -> Result<(), Box<dyn Error>> {
        // ANCHOR: sign_tx
        let secret = SecretKey::from_str(
            "5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1",
        )?;
        let wallet = WalletUnlocked::new_from_private_key(secret, None);

        // Set up a dummy transaction.
        let input_coin = Input::coin_signed(
            UtxoId::new(Bytes32::zeroed(), 0),
            Address::from_str(
                "0xf1e92c42b90934aa6372e30bc568a326f6e66a1a0288595e6e3fbd392a4f3e6e",
            )?,
            10000000,
            AssetId::from([0u8; 32]),
            TxPointer::default(),
            0,
            0,
        );

        let output_coin = Output::coin(
            Address::from_str(
                "0xc7862855b418ba8f58878db434b21053a61a2025209889cc115989e8040ff077",
            )?,
            1,
            AssetId::from([0u8; 32]),
        );

        let mut tx = Transaction::script(
            0,
            1000000,
            0,
            hex::decode("24400000")?,
            vec![],
            vec![input_coin],
            vec![output_coin],
            vec![],
        );

        // Sign the transaction.
        let signature = wallet.sign_transaction(&mut tx).await?;
        let message = unsafe { Message::from_bytes_unchecked(*tx.id()) };

        // Check if signature is what we expect it to be
        assert_eq!(signature, Signature::from_str("34482a581d1fe01ba84900581f5321a8b7d4ec65c3e7ca0de318ff8fcf45eb2c793c4b99e96400673e24b81b7aa47f042cad658f05a84e2f96f365eb0ce5a511")?);

        // Recover address that signed the transaction
        let recovered_address = signature.recover(&message)?;

        assert_eq!(wallet.address().hash(), recovered_address.hash());

        // Verify signature
        signature.verify(&recovered_address, &message)?;
        Ok(())
        // ANCHOR_END: sign_tx
    }

    #[tokio::test]
    async fn send_transfer_transactions() -> fuels_types::errors::Result<()> {
        // Setup two sets of coins, one for each wallet, each containing 1 coin with 1 amount.
        let mut wallet_1 = WalletUnlocked::new_random(None);
        let mut wallet_2 = WalletUnlocked::new_random(None).lock();

        let mut coins_1 = setup_single_asset_coins(wallet_1.address(), BASE_ASSET_ID, 1, 1000000);
        let coins_2 = setup_single_asset_coins(wallet_2.address(), BASE_ASSET_ID, 1, 1000000);

        coins_1.extend(coins_2);

        // Setup a provider and node with both set of coins.
        let (client, _) = setup_test_client(coins_1, vec![], None, None, None).await;
        let provider = Provider::new(client);

        wallet_1.set_provider(provider.clone());
        wallet_2.set_provider(provider);

        let wallet_1_initial_coins = wallet_1.get_coins(BASE_ASSET_ID).await?;
        let wallet_2_initial_coins = wallet_2.get_coins(BASE_ASSET_ID).await?;

        // Check initial wallet state.
        assert_eq!(wallet_1_initial_coins.len(), 1);
        assert_eq!(wallet_2_initial_coins.len(), 1);

        // Configure transaction parameters.
        let gas_price = 1;
        let gas_limit = 500_000;
        let maturity = 0;

        let tx_params = TxParameters {
            gas_price,
            gas_limit,
            maturity,
        };

        // Transfer 1 from wallet 1 to wallet 2.
        let (tx_id, _receipts) = wallet_1
            .transfer(wallet_2.address(), 1, Default::default(), tx_params)
            .await?;

        // Assert that the transaction was properly configured.
        let res = wallet_1
            .get_provider()?
            .get_transaction_by_id(&tx_id)
            .await?
            .unwrap();

        let script = res.transaction.as_script().cloned().unwrap();
        assert_eq!(script.limit(), gas_limit);
        assert_eq!(script.price(), gas_price);
        assert_eq!(*script.maturity(), maturity);

        let wallet_1_spendable_resources =
            wallet_1.get_spendable_resources(BASE_ASSET_ID, 0).await?;
        let wallet_1_all_coins = wallet_1.get_coins(BASE_ASSET_ID).await?;
        let wallet_2_all_coins = wallet_2.get_coins(BASE_ASSET_ID).await?;

        // wallet_1 has now only 1 spent coin (so 0 spendable)
        assert_eq!(wallet_1_spendable_resources.len(), 0);
        assert_eq!(wallet_1_all_coins.len(), 1);
        // Check that wallet two now has two coins.
        assert_eq!(wallet_2_all_coins.len(), 2);

        // Transferring more than balance should fail.
        let response = wallet_1
            .transfer(
                wallet_2.address(),
                2000000,
                Default::default(),
                TxParameters::default(),
            )
            .await;

        assert!(response.is_err());
        let wallet_2_coins = wallet_2.get_coins(BASE_ASSET_ID).await?;
        assert_eq!(wallet_2_coins.len(), 2); // Not changed
        Ok(())
    }

    #[tokio::test]
    async fn transfer_coins_with_change() -> fuels_types::errors::Result<()> {
        // Setup two sets of coins, one for each wallet, each containing 1 coin with 5 amounts each.
        let mut wallet_1 = WalletUnlocked::new_random(None);
        let mut wallet_2 = WalletUnlocked::new_random(None).lock();

        let mut coins_1 = setup_single_asset_coins(wallet_1.address(), BASE_ASSET_ID, 1, 5);
        let coins_2 = setup_single_asset_coins(wallet_2.address(), BASE_ASSET_ID, 1, 5);

        coins_1.extend(coins_2);

        let (client, _) = setup_test_client(coins_1, vec![], None, None, None).await;
        let provider = Provider::new(client);

        wallet_1.set_provider(provider.clone());
        wallet_2.set_provider(provider);

        let wallet_1_initial_coins = wallet_1.get_coins(BASE_ASSET_ID).await?;
        let wallet_2_initial_coins = wallet_2.get_coins(BASE_ASSET_ID).await?;

        assert_eq!(wallet_1_initial_coins.len(), 1);
        assert_eq!(wallet_2_initial_coins.len(), 1);

        // Transfer 2 from wallet 1 to wallet 2.
        let _receipts = wallet_1
            .transfer(
                wallet_2.address(),
                2,
                Default::default(),
                TxParameters::default(),
            )
            .await?;

        let wallet_1_final_coins = wallet_1.get_coins(BASE_ASSET_ID).await?;

        // Assert that we've sent 2 from wallet 1, resulting in an amount of 3 in wallet 1.
        let resulting_amount = wallet_1_final_coins.first().unwrap();
        assert_eq!(resulting_amount.amount, 3);

        let wallet_2_final_coins = wallet_2.get_coins(BASE_ASSET_ID).await?;
        assert_eq!(wallet_2_final_coins.len(), 2);

        // Check that wallet 2's amount is 7:
        // 5 initial + 2 that was sent to it.
        let total_amount: u64 = wallet_2_final_coins.iter().map(|c| c.amount).sum();
        assert_eq!(total_amount, 7);
        Ok(())
    }
}
