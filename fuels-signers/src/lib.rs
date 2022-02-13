mod wallet;

use fuel_core::service::{Config, FuelService};
use fuel_gql_client::client::FuelClient;

use provider::Provider;
use rand::{prelude::StdRng, Rng, RngCore, SeedableRng};
use secp256k1::SecretKey;
pub use wallet::Wallet;
pub mod provider;
pub mod signature;

use signature::Signature;

use async_trait::async_trait;
use fuel_tx::Transaction;
use fuel_types::Address;
use std::error::Error;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = Wallet;

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
    async fn sign_transaction(&self, message: &Transaction) -> Result<Signature, Self::Error>;

    /// Returns the signer's Fuel Address
    fn address(&self) -> Address;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fuel_tx::{Bytes32, Color, Input, Output, UtxoId};
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use secp256k1::SecretKey;

    use super::*;

    async fn setup_local_node() -> FuelClient {
        let srv = FuelService::new_node(Config::local_node()).await.unwrap();
        FuelClient::from(srv.bound_address)
    }

    fn setup_random_wallet(client: &FuelClient) -> Wallet {
        let mut rng = rand::thread_rng();
        let secret_seed = rng.gen::<[u8; 32]>();

        let secret =
            SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");

        let mut wallet = LocalWallet::new_from_private_key(secret).unwrap();

        let provider = Provider {
            client: client.clone(),
        };

        wallet.set_provider(provider);

        wallet
    }

    #[tokio::test]
    async fn sign_and_verify() {
        let mut rng = StdRng::seed_from_u64(2322u64);
        let mut secret_seed = [0u8; 32];
        rng.fill_bytes(&mut secret_seed);

        let secret =
            SecretKey::from_slice(&secret_seed).expect("Failed to generate random secret!");

        let wallet = LocalWallet::new_from_private_key(secret).unwrap();

        let message = "my message";

        let signature = wallet.sign_message(message.as_bytes()).await.unwrap();

        // Check if signature is what we expect it to be
        assert_eq!(signature.compact, Signature::from_str("0x8eeb238db1adea4152644f1cd827b552dfa9ab3f4939718bb45ca476d167c6512a656f4d4c7356bfb9561b14448c230c6e7e4bd781df5ee9e5999faa6495163d").unwrap().compact);

        // Recover address that signed the message
        let recovered_address = signature.recover(message).unwrap();

        assert_eq!(wallet.address, recovered_address);

        // Verify signature
        signature.verify(message, recovered_address).unwrap();
    }

    #[tokio::test]
    async fn sign_tx_and_verify() {
        let secret =
            SecretKey::from_str("5f70feeff1f229e4a95e1056e8b4d80d0b24b565674860cc213bdb07127ce1b1")
                .unwrap();

        let wallet = LocalWallet::new_from_private_key(secret).unwrap();

        let input_coin = Input::coin(
            UtxoId::new(Bytes32::zeroed(), 0),
            Address::from_str("0xf1e92c42b90934aa6372e30bc568a326f6e66a1a0288595e6e3fbd392a4f3e6e")
                .unwrap(),
            10000000,
            Color::from([0u8; 32]),
            0,
            0,
            vec![],
            vec![],
        );

        let output_coin = Output::coin(
            Address::from_str("0xc7862855b418ba8f58878db434b21053a61a2025209889cc115989e8040ff077")
                .unwrap(),
            1,
            Color::from([0u8; 32]),
        );

        let tx = Transaction::script(
            0,
            1000000,
            0,
            0,
            hex::decode("24400000").unwrap(),
            vec![],
            vec![input_coin],
            vec![output_coin],
            vec![],
        );

        let signature = wallet.sign_transaction(&tx).await.unwrap();

        // Check if signature is what we expect it to be
        assert_eq!(signature.compact, Signature::from_str("0xa1287a24af13fc102cb9e60988b558d5575d7870032f64bafcc2deda2c99125fb25eca55a29a169de156cb30700965e2b26278fcc7ad375bc720440ea50ba3cb").unwrap().compact);

        // Recover address that signed the transaction
        let recovered_address = signature.recover(&tx.id()).unwrap();

        assert_eq!(wallet.address, recovered_address);

        // Verify signature
        signature.verify(&tx.id(), recovered_address).unwrap();
    }

    #[tokio::test]
    async fn send_transaction() {
        // @todo Next:
        // 1. What about signatures / signing this transaction?
        // 2. Read more `fuel-core` tests to see what we can do about
        // having that `utxo` field in `transfer`. Can we just not have it?
        // How do we know which coin we can transfer?

        let node = setup_local_node().await;

        let wallet_1 = setup_random_wallet(&node);
        let wallet_2 = setup_random_wallet(&node);

        let initial_coins = wallet_2.get_coins().await.unwrap();

        assert_eq!(initial_coins.len(), 0);

        let _res = wallet_1
            .transfer(
                &wallet_2.address(),
                1,
                UtxoId::new(Bytes32::from([1_u8; 32]), 0),
            )
            .await
            .unwrap();

        let final_coins = wallet_2.get_coins().await.unwrap();
        assert_eq!(final_coins.len(), 1);
    }
}
