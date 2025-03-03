use crate::accounts_utils::try_provider_error;
use crate::provider::Provider;
use crate::wallet::Wallet;
use crate::{Account, ViewOnlyAccount};
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
use google_cloud_kms::client::Client;
use google_cloud_kms::grpc::kms::v1::crypto_key_version::CryptoKeyVersionAlgorithm::EcSignSecp256k1Sha256;
use google_cloud_kms::grpc::kms::v1::digest::Digest::Sha256;
use google_cloud_kms::grpc::kms::v1::{AsymmetricSignRequest, Digest, GetPublicKeyRequest};
use k256::{
    ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey},
    pkcs8::DecodePublicKey,
    PublicKey as K256PublicKey,
};

const GOOGLE_KMS_ERROR_PREFIX: &str = "Google KMS Error";

/// A wallet implementation that uses Google Cloud KMS for signing
#[derive(Clone, Debug)]
pub struct GoogleWallet {
    view_account: Wallet,
    kms_key: GcpKey,
}

#[derive(Clone, Debug)]
pub struct GcpKey {
    key_path: String,
    client: Client,
    public_key_pem: String,
    fuel_address: Bech32Address,
}

#[derive(Debug, Clone)]
pub struct CryptoKeyVersionName {
    pub project_id: String,
    pub location: String,
    pub key_ring: String,
    pub key_id: String,
    pub key_version: String,
}

impl CryptoKeyVersionName {
    pub fn new(
        project_id: impl Into<String>,
        location: impl Into<String>,
        key_ring: impl Into<String>,
        key_id: impl Into<String>,
        key_version: impl Into<String>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            location: location.into(),
            key_ring: key_ring.into(),
            key_id: key_id.into(),
            key_version: key_version.into(),
        }
    }
}

impl std::fmt::Display for CryptoKeyVersionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}/cryptoKeyVersions/{}",
            self.project_id, self.location, self.key_ring, self.key_id, self.key_version
        )
    }
}

impl GcpKey {
    pub fn key_path(&self) -> &String {
        &self.key_path
    }

    pub fn public_key(&self) -> &String {
        &self.public_key_pem
    }

    pub fn fuel_address(&self) -> &Bech32Address {
        &self.fuel_address
    }

    /// Creates a new GcpKey from a Google Cloud KMS key path.
    /// The key_path should be in the format:
    /// projects/{project}/locations/{location}/keyRings/{key_ring}/cryptoKeys/{key}/cryptoKeyVersions/{version}
    pub async fn new(key_path: String, client: &Client) -> Result<Self> {
        let public_key_pem = Self::retrieve_public_key(client, &key_path).await?;
        let fuel_address = Self::derive_fuel_address(&public_key_pem)?;
        Ok(Self {
            key_path,
            client: client.clone(),
            public_key_pem,
            fuel_address,
        })
    }

    /// Retrieves the public key PEM from Google Cloud KMS and verifies the key algorithm.
    async fn retrieve_public_key(client: &Client, key_path: &str) -> Result<String> {
        let request = GetPublicKeyRequest {
            name: key_path.to_string(),
        };

        let response = client
            .get_public_key(request, None)
            .await
            .map_err(|e| format_gcp_error(format!("Failed to get public key: {}", e)))?;

        dbg!(&response);

        // Check that the key algorithm matches EC_SIGN_SECP256K1_SHA256.
        if response.algorithm != EcSignSecp256k1Sha256 as i32 {
            return Err(Error::Other(format!(
                "{GOOGLE_KMS_ERROR_PREFIX}: Invalid key algorithm: {}, expected EC_SIGN_SECP256K1_SHA256",
                response.algorithm
            )));
        }
        Ok(response.pem)
    }

    /// Derives a Fuel address from the PEM-encoded public key.
    fn derive_fuel_address(pem: &str) -> Result<Bech32Address> {
        let k256_key = K256PublicKey::from_public_key_pem(pem).map_err(|_| {
            Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid PEM encoding"))
        })?;
        let fuel_public_key = PublicKey::from(k256_key);
        Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_public_key.hash()))
    }

    /// Requests a signature from Google Cloud KMS.
    async fn request_gcp_signature(&self, message: Message) -> Result<Vec<u8>> {
        let digest = Digest {
            digest: Some(Sha256(message.as_ref().to_vec())),
        };

        let request = AsymmetricSignRequest {
            name: self.key_path.clone(),
            digest: Some(digest),
            digest_crc32c: None,
            ..AsymmetricSignRequest::default()
        };

        let response = self
            .client
            .asymmetric_sign(request, None)
            .await
            .map_err(|e| format_gcp_error(format!("Signing failed: {}", e)))?;

        if response.signature.is_empty() {
            return Err(Error::Other(format!(
                "{GOOGLE_KMS_ERROR_PREFIX}: Empty signature response"
            )));
        }
        Ok(response.signature)
    }

    /// Signs the given message by requesting a signature from Google Cloud KMS,
    /// then normalizing the DER signature and determining the recovery ID.
    async fn sign_message(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_gcp_signature(message).await?;
        let (normalized_sig, recovery_id) = self.normalize_signature(&signature_der, message)?;
        Ok(self.convert_to_fuel_signature(normalized_sig, recovery_id))
    }

    /// Normalizes a DER signature and determines the recovery ID.
    fn normalize_signature(
        &self,
        signature_der: &[u8],
        message: Message,
    ) -> Result<(K256Signature, RecoveryId)> {
        let signature = K256Signature::from_der(signature_der).map_err(|_| {
            Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid DER signature"))
        })?;

        let normalized_sig = signature.normalize_s().unwrap_or(signature);
        let recovery_id = self.determine_recovery_id(&normalized_sig, message)?;

        Ok((normalized_sig, recovery_id))
    }

    /// Determines the correct recovery ID for the signature by comparing
    /// the recovered public key with the expected public key.
    fn determine_recovery_id(&self, sig: &K256Signature, message: Message) -> Result<RecoveryId> {
        let recid_even = RecoveryId::new(false, false);
        let recid_odd = RecoveryId::new(true, false);

        let expected_pubkey =
            K256PublicKey::from_public_key_pem(&self.public_key_pem).map_err(|_| {
                Error::Other(format!(
                    "{GOOGLE_KMS_ERROR_PREFIX}: Invalid cached public key"
                ))
            })?;
        let expected_verifying_key: VerifyingKey = expected_pubkey.into();

        let recovered_even = VerifyingKey::recover_from_prehash(&*message, sig, recid_even);
        let recovered_odd = VerifyingKey::recover_from_prehash(&*message, sig, recid_odd);

        if recovered_even
            .map(|r| r == expected_verifying_key)
            .unwrap_or(false)
        {
            Ok(recid_even)
        } else if recovered_odd
            .map(|r| r == expected_verifying_key)
            .unwrap_or(false)
        {
            Ok(recid_odd)
        } else {
            Err(Error::Other(format!(
                "{GOOGLE_KMS_ERROR_PREFIX}: Invalid signature (could not recover correct public key)"
            )))
        }
    }

    /// Converts the DER signature and recovery ID to a Fuel-compatible signature.
    fn convert_to_fuel_signature(
        &self,
        signature: K256Signature,
        recovery_id: RecoveryId,
    ) -> Signature {
        let recovery_byte = recovery_id.is_y_odd() as u8;
        let mut bytes: [u8; 64] = signature.to_bytes().into();
        bytes[32] = (recovery_byte << 7) | (bytes[32] & 0x7F);
        Signature::from_bytes(bytes)
    }
}

impl GoogleWallet {
    /// Creates a new GoogleWallet with the given KMS key path.
    pub async fn with_kms_key(
        key_path: CryptoKeyVersionName,
        google_client: &Client,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let kms_key = GcpKey::new(key_path.to_string(), google_client).await?;
        Ok(Self {
            view_account: Wallet::from_address(kms_key.fuel_address.clone(), provider),
            kms_key,
        })
    }
    pub fn address(&self) -> &Bech32Address {
        &self.kms_key.fuel_address
    }
    pub fn provider(&self) -> Option<&Provider> {
        self.view_account.provider()
    }
}

#[async_trait::async_trait]
impl Signer for GoogleWallet {
    async fn sign(&self, message: Message) -> Result<Signature> {
        self.kms_key.sign_message(message).await
    }

    fn address(&self) -> &Bech32Address {
        self.address()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ViewOnlyAccount for GoogleWallet {
    fn address(&self) -> &Bech32Address {
        &self.kms_key.fuel_address
    }

    fn try_provider(&self) -> Result<&Provider> {
        self.provider().ok_or_else(try_provider_error)
    }

    async fn get_asset_inputs_for_amount(
        &self,
        asset_id: AssetId,
        amount: u64,
        excluded_coins: Option<Vec<CoinTypeId>>,
    ) -> Result<Vec<Input>> {
        self.view_account
            .get_asset_inputs_for_amount(asset_id, amount, excluded_coins)
            .await
    }
}

#[async_trait::async_trait]
impl Account for GoogleWallet {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;
        Ok(())
    }
}

fn format_gcp_error(err: impl std::fmt::Display) -> Error {
    Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: {err}"))
}
