use aws_sdk_kms::{
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
    Client as AwsClient,
};
use fuel_crypto::{Message, PublicKey, Signature};
use fuel_types::AssetId;
use fuels_core::{
    traits::Signer,
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        coin_type_id::CoinTypeId,
        errors::{Error, Result},
        input::Input,
        transaction_builders::TransactionBuilder,
    },
};
use k256::{
    ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey},
    pkcs8::DecodePublicKey,
    PublicKey as K256PublicKey,
};

use crate::{provider::Provider, wallet::Wallet, Account, ViewOnlyAccount};

const AWS_KMS_ERROR_PREFIX: &str = "AWS KMS Error";

#[derive(Clone, Debug)]
pub struct AwsWallet {
    wallet: Wallet,
    kms_data: KmsData,
}

#[derive(Clone, Debug)]
pub struct KmsData {
    id: String,
    client: AwsClient,
    public_key: Vec<u8>,
    pub address: Bech32Address,
}

impl KmsData {
    pub async fn new(id: String, client: AwsClient) -> anyhow::Result<Self> {
        Self::validate_key_type(&client, &id).await?;
        let public_key = Self::fetch_public_key(&client, &id).await?;
        let address = Self::create_bech32_address(&public_key)?;

        Ok(Self {
            id,
            client,
            public_key,
            address,
        })
    }

    async fn validate_key_type(client: &AwsClient, key_id: &str) -> anyhow::Result<()> {
        let key_spec = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await?
            .key_spec;

        match key_spec {
            Some(KeySpec::EccSecgP256K1) => Ok(()),
            other => anyhow::bail!(
                "{}: Invalid key type, expected EccSecgP256K1, got {:?}",
                AWS_KMS_ERROR_PREFIX,
                other
            ),
        }
    }

    async fn fetch_public_key(client: &AwsClient, key_id: &str) -> anyhow::Result<Vec<u8>> {
        let response = client.get_public_key().key_id(key_id).send().await?;

        let public_key = response
            .public_key()
            .ok_or_else(|| anyhow::anyhow!("{}: No public key returned", AWS_KMS_ERROR_PREFIX))?;

        Ok(public_key.clone().into_inner())
    }

    fn create_bech32_address(public_key_bytes: &[u8]) -> anyhow::Result<Bech32Address> {
        let k256_public_key = K256PublicKey::from_public_key_der(public_key_bytes)
            .map_err(|_| anyhow::anyhow!("{}: Invalid DER public key", AWS_KMS_ERROR_PREFIX))?;

        let public_key = PublicKey::from(k256_public_key);
        let fuel_address = public_key.hash();

        Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_address))
    }

    async fn sign_message(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_signature(message).await?;
        let (signature, recovery_id) = self.process_signature(&signature_der, message)?;

        Ok(self.create_fuel_signature(signature, recovery_id))
    }

    async fn request_signature(&self, message: Message) -> Result<Vec<u8>> {
        let reply = self
            .client
            .sign()
            .key_id(&self.id)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message_type(MessageType::Digest)
            .message(Blob::new(*message))
            .send()
            .await
            .map_err(|e| {
                Error::Other(format!("{}: Failed to sign: {:?}", AWS_KMS_ERROR_PREFIX, e))
            })?;

        reply
            .signature
            .map(|sig| sig.into_inner())
            .ok_or_else(|| Error::Other(format!("{}: No signature returned", AWS_KMS_ERROR_PREFIX)))
    }

    fn process_signature(
        &self,
        signature_der: &[u8],
        message: Message,
    ) -> Result<(K256Signature, RecoveryId)> {
        let sig = K256Signature::from_der(signature_der).map_err(|_| {
            Error::Other(format!("{}: Invalid DER signature", AWS_KMS_ERROR_PREFIX))
        })?;
        let sig = sig.normalize_s().unwrap_or(sig);

        let recovery_id = self.determine_recovery_id(&sig, message)?;

        Ok((sig, recovery_id))
    }

    fn determine_recovery_id(&self, sig: &K256Signature, message: Message) -> Result<RecoveryId> {
        let recid1 = RecoveryId::new(false, false);
        let recid2 = RecoveryId::new(true, false);

        let correct_public_key = K256PublicKey::from_public_key_der(&self.public_key)
            .map_err(|_| {
                Error::Other(format!(
                    "{}: Invalid cached public key",
                    AWS_KMS_ERROR_PREFIX
                ))
            })?
            .into();

        let rec1 = VerifyingKey::recover_from_prehash(&*message, sig, recid1);
        let rec2 = VerifyingKey::recover_from_prehash(&*message, sig, recid2);

        if rec1.map(|r| r == correct_public_key).unwrap_or(false) {
            Ok(recid1)
        } else if rec2.map(|r| r == correct_public_key).unwrap_or(false) {
            Ok(recid2)
        } else {
            Err(Error::Other(format!(
                "{}: Invalid signature (reduced-x form coordinate)",
                AWS_KMS_ERROR_PREFIX
            )))
        }
    }

    fn create_fuel_signature(
        &self,
        signature: K256Signature,
        recovery_id: RecoveryId,
    ) -> Signature {
        debug_assert!(
            !recovery_id.is_x_reduced(),
            "reduced-x form coordinates should be caught earlier"
        );

        let v = recovery_id.is_y_odd() as u8;
        let mut signature_bytes = <[u8; 64]>::from(signature.to_bytes());
        signature_bytes[32] = (v << 7) | (signature_bytes[32] & 0x7f);

        Signature::from_bytes(signature_bytes)
    }
}

impl AwsWallet {
    pub async fn from_kms_key_id(
        kms_key_id: String,
        provider: Option<Provider>,
    ) -> anyhow::Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = AwsClient::new(&config);

        let kms_data = KmsData::new(kms_key_id, client).await?;
        println!("Fuel address: {}", kms_data.address);

        Ok(Self {
            wallet: Wallet::from_address(kms_data.address.clone(), provider),
            kms_data,
        })
    }

    pub fn address(&self) -> &Bech32Address {
        self.wallet.address()
    }

    pub fn provider(&self) -> Option<&Provider> {
        self.wallet.provider()
    }
}

#[async_trait::async_trait]
impl Signer for AwsWallet {
    async fn sign(&self, message: Message) -> Result<Signature> {
        self.kms_data.sign_message(message).await
    }

    fn address(&self) -> &Bech32Address {
        &self.kms_data.address
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ViewOnlyAccount for AwsWallet {
    fn address(&self) -> &Bech32Address {
        self.wallet.address()
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.wallet.provider().ok_or_else(|| {
            Error::Other("No provider available. Make sure to use `set_provider`".to_owned())
        })
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        self.wallet
            .get_asset_inputs_for_amount(asset_id, amount, excluded_coins)
            .await
    }
}

#[async_trait::async_trait]
impl Account for AwsWallet {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;
        Ok(())
    }
}

//
// // Integration tests using the LocalStack KMS
// // #[cfg(test)]
// // mod tests {
// //     use super::*;
// //     use fuel_crypto::Message;
// //     use std::env;
// //
// //     #[tokio::test]
// //     async fn test_kms_wallet_with_localstack() -> anyhow::Result<()> {
// //         // Start LocalStack
// //         let kms = KmsTestContainer::default().with_show_logs(true);
// //         let kms_process = kms.start().await?;
// //
// //         // Create a new KMS key
// //         let test_key = kms_process.create_key().await?;
// //
// //         // Set required environment variables
// //         env::set_var("AWS_ACCESS_KEY_ID", "test");
// //         env::set_var("AWS_SECRET_ACCESS_KEY", "test");
// //         env::set_var("AWS_REGION", "us-east-1");
// //         env::set_var("AWS_ENDPOINT_URL", test_key.url);
// //
// //         // Create KMS wallet
// //         let wallet = KMSWallet::from_kms_key_id(test_key.id, None).await?;
// //
// //         // Test signing
// //         let message = Message::new([1u8; 32]);
// //         let signature = wallet.sign(message).await?;
// //
// //         // Verify the signature
// //         let public_key = wallet.address().hash();
// //         // assert!(signature.verify(&message, &public_key));
// //
// //         Ok(())
// //     }
// //
// //     #[tokio::test]
// //     async fn test_multiple_signatures() -> anyhow::Result<()> {
// //         let kms = KmsTestContainer::default().with_show_logs(false);
// //         let kms_process = kms.start().await?;
// //         let test_key = kms_process.create_key().await?;
// //
// //         env::set_var("AWS_ACCESS_KEY_ID", "test");
// //         env::set_var("AWS_SECRET_ACCESS_KEY", "test");
// //         env::set_var("AWS_REGION", "us-east-1");
// //         env::set_var("AWS_ENDPOINT_URL", test_key.url);
// //
// //         let wallet = KMSWallet::from_kms_key_id(test_key.id, None).await?;
// //
// //         // Sign multiple messages
// //         for i in 0..5 {
// //             let message = Message::new([i as u8; 32]);
// //             let signature = wallet.sign(message).await?;
// //             // assert!(signature.verify(&message, &wallet.address().hash()));
// //         }
// //
// //         Ok(())
// //     }
// //
// //     #[tokio::test]
// //     async fn test_error_handling() -> anyhow::Result<()> {
// //         // Start LocalStack
// //         let kms = KmsTestContainer::default().with_show_logs(false);
// //         let kms_process = kms.start().await?;
// //
// //         // Set required environment variables first
// //         std::env::set_var("AWS_ACCESS_KEY_ID", "test");
// //         std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
// //         std::env::set_var("AWS_REGION", "us-east-1");
// //         std::env::set_var("AWS_ENDPOINT_URL", &kms_process.url);
// //
// //         // Test 1: Invalid key ID
// //         {
// //             let result = KMSWallet::from_kms_key_id("invalid-key-id".to_string(), None).await;
// //             dbg!(&result);
// //             assert!(result.is_err());
// //             let err = result.unwrap_err().to_string();
// //             println!("Invalid key ID error: {}", err);
// //             assert!(err.contains("AWS KMS Error")); // Check for our error prefix
// //         }
// //
// //         // Test 2: Wrong key spec
// //         {
// //             let response = kms_process
// //                 .client
// //                 .create_key()
// //                 .key_usage(aws_sdk_kms::types::KeyUsageType::SignVerify)
// //                 .key_spec(aws_sdk_kms::types::KeySpec::Rsa2048) // Wrong key spec
// //                 .send()
// //                 .await?;
// //
// //             let key_id = response
// //                 .key_metadata
// //                 .and_then(|metadata| metadata.arn)
// //                 .ok_or_else(|| anyhow::anyhow!("Key ARN missing from response"))?;
// //
// //             let result = KMSWallet::from_kms_key_id(key_id, None).await;
// //             println!("Wrong key spec error: {:?}", result);
// //             assert!(result.is_err());
// //             let err = result.unwrap_err().to_string();
// //             assert!(err.contains("Invalid key type") || err.contains("key_spec"));
// //         }
// //
// //         // Test 3: Invalid endpoint
// //         {
// //             // Set invalid endpoint
// //             std::env::set_var("AWS_ENDPOINT_URL", "http://invalid-endpoint:4566");
// //
// //             let result = KMSWallet::from_kms_key_id("any-key-id".to_string(), None).await;
// //             assert!(result.is_err());
// //             let err = result.unwrap_err().to_string();
// //             println!("Invalid endpoint error: {}", err);
// //             assert!(err.contains("AWS KMS Error") || err.contains("endpoint"));
// //         }
// //
// //         Ok(())
// //     }
// //
// //     // Helper function to print environment variables (useful for debugging)
// //     fn print_aws_env() {
// //         println!("AWS Environment Variables:");
// //         println!("AWS_ACCESS_KEY_ID: {:?}", env::var("AWS_ACCESS_KEY_ID"));
// //         println!("AWS_SECRET_ACCESS_KEY: {:?}", env::var("AWS_SECRET_ACCESS_KEY"));
// //         println!("AWS_REGION: {:?}", env::var("AWS_REGION"));
// //         println!("AWS_ENDPOINT_URL: {:?}", env::var("AWS_ENDPOINT_URL"));
// //     }
// // }
//
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use fuel_crypto::{Message, PublicKey};
//     use std::env;
//     use std::str::FromStr;
//
//     // Helper function to set up test environment
//     async fn setup_test_environment() -> anyhow::Result<(KmsTestProcess, KmsTestKey)> {
//         let kms = KmsTestContainer::default().with_show_logs(false);
//         let kms_process = kms.start().await?;
//         let test_key = kms_process.create_key().await?;
//
//         // Set required environment variables
//         env::set_var("AWS_ACCESS_KEY_ID", "test");
//         env::set_var("AWS_SECRET_ACCESS_KEY", "test");
//         env::set_var("AWS_REGION", "us-east-1");
//         env::set_var("AWS_ENDPOINT_URL", &test_key.url);
//
//         Ok((kms_process, test_key))
//     }
//
//     #[tokio::test]
//     async fn test_wallet_creation_and_basic_operations() -> anyhow::Result<()> {
//         let (kms_process,test_key) = setup_test_environment().await?;
//
//         // Test wallet creation
//         let wallet = AWSWallet::from_kms_key_id(test_key.id, None).await?;
//
//         // Verify address format
//         assert!(wallet.address().to_string().starts_with("fuel"));
//
//         // Verify provider behavior
//         assert!(wallet.provider().is_none());
//         assert!(wallet.try_provider().is_err());
//
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_message_signing_and_verification() -> anyhow::Result<()> {
//         let (kms_process, test_key) = setup_test_environment().await?;
//         let wallet = AWSWallet::from_kms_key_id(test_key.id, None).await?;
//
//         // Test signing with different message types
//         let test_cases = vec![
//             [0u8; 32],  // Zero message
//             [1u8; 32],  // All ones
//             [255u8; 32],  // All max values
//             [123u8; 32],  // Random values
//         ];
//
//         for msg_bytes in test_cases {
//             let message = Message::new(msg_bytes);
//             let signature = wallet.sign(message).await?;
//
//             // Verify the signature using the wallet's public key
//             let public_key = PublicKey::from_str(&wallet.address().hash().to_string())?;
//
//             // assert!(signature.verify(&message, &public_key));
//         }
//
//         Ok(())
//     }
//
//     // #[tokio::test]
//     // async fn test_concurrent_signing() -> anyhow::Result<()> {
//     //     let (_, test_key) = setup_test_environment().await?;
//     //     let wallet = KMSWallet::from_kms_key_id(test_key.id, None).await?;
//     //
//     //     // Create multiple signing tasks
//     //     let mut handles = vec![];
//     //     for i in 0..5 {
//     //         let wallet_clone = wallet.clone();
//     //         let handle = tokio::spawn(async move {
//     //             let message = Message::new([i as u8; 32]);
//     //             wallet_clone.sign(message).await
//     //         });
//     //         handles.push(handle);
//     //     }
//     //
//     //     // Wait for all signatures and verify them
//     //     for handle in handles {
//     //         let signature = handle.await??;
//     //         assert!(signature.verify(
//     //             &Message::new([0u8; 32]),
//     //             &PublicKey::from_bytes(wallet.address().hash().as_ref())?
//     //         ));
//     //     }
//     //
//     //     Ok(())
//     // }
//     //
//     // #[tokio::test]
//     // async fn test_error_cases() -> anyhow::Result<()> {
//     //     let (kms_process, _) = setup_test_environment().await?;
//     //
//     //     // Test 1: Invalid key ID
//     //     let result = KMSWallet::from_kms_key_id("invalid-key-id".to_string(), None).await;
//     //     assert!(result.is_err());
//     //     assert!(result.unwrap_err().to_string().contains("AWS KMS Error"));
//     //
//     //     // Test 2: Wrong key spec
//     //     let response = kms_process
//     //         .client
//     //         .create_key()
//     //         .key_usage(aws_sdk_kms::types::KeyUsageType::SignVerify)
//     //         .key_spec(aws_sdk_kms::types::KeySpec::Rsa2048)
//     //         .send()
//     //         .await?;
//     //
//     //     let key_id = response
//     //         .key_metadata
//     //         .and_then(|metadata| metadata.arn)
//     //         .ok_or_else(|| anyhow::anyhow!("Key ARN missing from response"))?;
//     //
//     //     let result = KMSWallet::from_kms_key_id(key_id, None).await;
//     //     assert!(result.is_err());
//     //     assert!(result.unwrap_err().to_string().contains("Invalid key type"));
//     //
//     //     // Test 3: Missing environment variables
//     //     env::remove_var("AWS_REGION");
//     //     let result = KMSWallet::from_kms_key_id("any-key-id".to_string(), None).await;
//     //     assert!(result.is_err());
//     //
//     //     Ok(())
//     // }
//     //
//     // #[tokio::test]
//     // async fn test_address_consistency() -> anyhow::Result<()> {
//     //     let (_, test_key) = setup_test_environment().await?;
//     //     let wallet = KMSWallet::from_kms_key_id(test_key.id.clone(), None).await?;
//     //
//     //     // Create another wallet instance with the same key
//     //     let wallet2 = KMSWallet::from_kms_key_id(test_key.id, None).await?;
//     //
//     //     // Verify addresses match
//     //     assert_eq!(wallet.address(), wallet2.address());
//     //     // assert_eq!(wallet.address(), wallet.kms_data.cached_address);
//     //
//     //     Ok(())
//     // }
//     //
//     //
//     //
//     // #[tokio::test]
//     // async fn test_asset_inputs() -> anyhow::Result<()> {
//     //     let (_, test_key) = setup_test_environment().await?;
//     //     let wallet = KMSWallet::from_kms_key_id(test_key.id, None).await?;
//     //
//     //     // Test getting asset inputs (should fail without provider)
//     //     let result = wallet
//     //         .get_asset_inputs_for_amount(AssetId::default(), 100, None)
//     //         .await;
//     //     assert!(result.is_err());
//     //
//     //     Ok(())
//     // }
// }
