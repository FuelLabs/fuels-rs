// use crate::accounts_utils::try_provider_error;
// use crate::kms::google::client::GoogleClient;
// use crate::provider::Provider;
// use crate::wallet::Wallet;
// use crate::{Account, ViewOnlyAccount};
// use fuel_crypto::{Message, PublicKey, Signature};
// use fuel_types::AssetId;
// use google_cloud_kms::grpc::kms::v1::crypto_key_version::CryptoKeyVersionAlgorithm::EcSignSecp256k1Sha256;
// use google_cloud_kms::grpc::kms::v1::{AsymmetricSignRequest, GetPublicKeyRequest};
// use google_cloud_kms::grpc::kms::v1::digest::Digest::Sha256;
// use fuels_core::{
//     traits::Signer,
//     types::{
//         bech32::{Bech32Address, FUEL_BECH32_HRP},
//         coin_type_id::CoinTypeId,
//         errors::{Error, Result},
//         input::Input,
//         transaction_builders::TransactionBuilder,
//     },
// };
// use k256::{
//     ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey},
//     pkcs8::DecodePublicKey,
//     PublicKey as K256PublicKey,
// };
// use prost::Message as ProstMessage;
// use tonic::transport::Channel;
//
// /// Error prefix for Google KMS related operations
// const GOOGLE_KMS_ERROR_PREFIX: &str = "Google KMS Error";
//
// /// A wallet implementation that uses Google Cloud KMS for signing
// #[derive(Clone, Debug)]
// pub struct GoogleWallet {
//     view_account: Wallet,
//     kms_key: GcpKey,
// }
//
// /// Represents a Google Cloud KMS key with Fuel-compatible address
// #[derive(Clone, Debug)]
// pub struct GcpKey {
//     key_path: String,
//     client: GoogleClient,
//     public_key_pem: String,
//     fuel_address: Bech32Address,
// }
//
// impl GcpKey {
//     pub fn key_path(&self) -> &String {
//         &self.key_path
//     }
//
//     pub fn public_key(&self) -> &String {
//         &self.public_key_pem
//     }
//
//     pub fn fuel_address(&self) -> &Bech32Address {
//         &self.fuel_address
//     }
//
//     /// Creates a new GcpKey from a Google Cloud KMS key path
//     /// The key_path should be in the format: projects/{project}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{key}/cryptoKeyVersions/{version}
//     pub async fn new(key_path: String, client: &GoogleClient) -> Result<Self> {
//         // Validate the key spec by attempting to retrieve the public key
//         let public_key_pem = Self::retrieve_public_key(client, &key_path).await?;
//
//         // Derive the Fuel address from the public key
//         let fuel_address = Self::derive_fuel_address(&public_key_pem)?;
//
//         Ok(Self {
//             key_path,
//             client: client.clone(),
//             public_key_pem,
//             fuel_address,
//         })
//     }
//
//     /// Retrieves the public key from Google Cloud KMS
//     async fn retrieve_public_key(client: &GoogleClient, key_path: &str) -> Result<String> {
//         let request = GetPublicKeyRequest {
//             name: key_path.to_string(),
//         };
//
//         let response = client
//             .inner()
//             .get_public_key(request, None)
//             .await
//             .map_err(|e| format_gcp_error(format!("Failed to get public key: {}", e)))?;
//
//         let key_spec = response.algorithm;
//         // Check if the key is EC_SIGN_SECP256K1_SHA256
//         if key_spec != EcSignSecp256k1Sha256 as i32 {
//             return Err(Error::Other(format!(
//                 "{GOOGLE_KMS_ERROR_PREFIX}: Invalid key algorithm: {}, expected EC_SIGN_SECP256K1_SHA256",
//                 key_spec
//             )));
//         }
//
//         Ok(response.pem)
//     }
//
//     /// Derives a Fuel address from a public key in PEM format
//     fn derive_fuel_address(pem: &str) -> Result<Bech32Address> {
//         let k256_key = K256PublicKey::from_public_key_pem(pem)
//             .map_err(|_| Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid PEM encoding")))?;
//
//         let fuel_public_key = PublicKey::from(k256_key);
//         Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_public_key.hash()))
//     }
//
//     /// Signs a message using the Google Cloud KMS key
//     async fn sign_message(&self, message: Message) -> Result<Signature> {
//         let signature_der = self.request_gcp_signature(message).await?;
//         let (sig, recovery_id) = self.normalize_signature(&signature_der, message)?;
//
//         Ok(self.convert_to_fuel_signature(sig, recovery_id))
//     }
//
//     /// Requests a signature from Google Cloud KMS
//     async fn request_gcp_signature(&self, message: Message) -> Result<Vec<u8>> {
//         let digest = GcpDigest {
//             digest: vec![
//                 Sha256(message.as_ref().to_vec()),
//             ],
//         };
//
//         let request = AsymmetricSignRequest {
//             name: self.key_path.clone(),
//             digest: Some(digest),
//             digest_crc32c: None,
//             data: vec![],
//             data_crc32c: None,
//         };
//
//         let response = self
//             .client
//             .inner()
//             .asymmetric_sign(request, None)
//             .await
//             .map_err(|e| format_gcp_error(format!("Signing failed: {}", e)))?;
//
//         let signature = response.signature;
//         if signature.is_empty() {
//             return Err(Error::Other(format!(
//                 "{GOOGLE_KMS_ERROR_PREFIX}: Empty signature response"
//             )));
//         }
//
//         Ok(signature)
//     }
//
//     /// Normalizes a DER signature and determines the recovery ID
//     fn normalize_signature(
//         &self,
//         signature_der: &[u8],
//         message: Message,
//     ) -> Result<(K256Signature, RecoveryId)> {
//         let signature = K256Signature::from_der(signature_der)
//             .map_err(|_| Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid DER signature")))?;
//
//         // Ensure the signature is in normalized form (low-S value)
//         let normalized_sig = signature.normalize_s().unwrap_or(signature);
//         let recovery_id = self.determine_recovery_id(&normalized_sig, message)?;
//
//         Ok((normalized_sig, recovery_id))
//     }
//
//     /// Determines the correct recovery ID for the signature
//     fn determine_recovery_id(&self, sig: &K256Signature, message: Message) -> Result<RecoveryId> {
//         let recid_even = RecoveryId::new(false, false);
//         let recid_odd = RecoveryId::new(true, false);
//
//         // Get the expected public key
//         let expected_pubkey = K256PublicKey::from_public_key_pem(&self.public_key_pem)
//             .map_err(|_| {
//                 Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid cached public key"))
//             })?
//             .into();
//
//         // Try recovery with each recovery ID
//         let recovered_even = VerifyingKey::recover_from_prehash(&*message, sig, recid_even);
//         let recovered_odd = VerifyingKey::recover_from_prehash(&*message, sig, recid_odd);
//
//         if recovered_even
//             .map(|r| r == expected_pubkey)
//             .unwrap_or(false)
//         {
//             Ok(recid_even)
//         } else if recovered_odd.map(|r| r == expected_pubkey).unwrap_or(false) {
//             Ok(recid_odd)
//         } else {
//             Err(Error::Other(format!(
//                 "{GOOGLE_KMS_ERROR_PREFIX}: Invalid signature (could not recover correct public key)"
//             )))
//         }
//     }
//
//     /// Converts a k256 signature to a Fuel signature format
//     fn convert_to_fuel_signature(
//         &self,
//         signature: K256Signature,
//         recovery_id: RecoveryId,
//     ) -> Signature {
//         let recovery_byte = recovery_id.is_y_odd() as u8;
//         let mut bytes: [u8; 64] = signature.to_bytes().into();
//         bytes[32] = (recovery_byte << 7) | (bytes[32] & 0x7F);
//         Signature::from_bytes(bytes)
//     }
//
//     /// Returns the Fuel address associated with this key
//     pub fn address(&self) -> &Bech32Address {
//         &self.fuel_address
//     }
// }
//
// impl GoogleWallet {
//     /// Creates a new GoogleWallet with the given KMS key path
//     /// The key_path should be in the format: projects/{project}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{key}/cryptoKeyVersions/{version}
//     pub async fn with_kms_key(
//         key_path: impl Into<String>,
//         google_client: &GoogleClient,
//         provider: Option<Provider>,
//     ) -> Result<Self> {
//         let kms_key = GcpKey::new(key_path.into(), google_client).await?;
//
//         Ok(Self {
//             view_account: Wallet::from_address(kms_key.fuel_address.clone(), provider),
//             kms_key,
//         })
//     }
//
//     /// Returns the Fuel address associated with this wallet
//     pub fn address(&self) -> &Bech32Address {
//         &self.kms_key.fuel_address
//     }
//
//     /// Returns the provider associated with this wallet, if any
//     pub fn provider(&self) -> Option<&Provider> {
//         self.view_account.provider()
//     }
// }
//
// #[async_trait::async_trait]
// impl Signer for GoogleWallet {
//     async fn sign(&self, message: Message) -> Result<Signature> {
//         self.kms_key.sign_message(message).await
//     }
//
//     fn address(&self) -> &Bech32Address {
//         self.address()
//     }
// }
//
// #[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
// impl ViewOnlyAccount for GoogleWallet {
//     fn address(&self) -> &Bech32Address {
//         &self.kms_key.fuel_address
//     }
//
//     fn try_provider(&self) -> Result<&Provider> {
//         self.provider().ok_or_else(try_provider_error)
//     }
//
//     async fn get_asset_inputs_for_amount(
//         &self,
//         asset_id: AssetId,
//         amount: u64,
//         excluded_coins: Option<Vec<CoinTypeId>>,
//     ) -> Result<Vec<Input>> {
//         self.view_account
//             .get_asset_inputs_for_amount(asset_id, amount, excluded_coins)
//             .await
//     }
// }
//
// #[async_trait::async_trait]
// impl Account for GoogleWallet {
//     fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
//         tb.add_signer(self.clone())?;
//         Ok(())
//     }
// }
//
// fn format_gcp_error(err: impl std::fmt::Display) -> Error {
//     Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: {err}"))
// }
