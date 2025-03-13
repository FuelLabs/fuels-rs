use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, Signature};
use fuels_core::traits::Signer;
use fuels_core::types::{
    bech32::{Bech32Address, FUEL_BECH32_HRP},
    errors::{Error, Result},
};
pub use google_cloud_kms;
use google_cloud_kms::client::Client;
use google_cloud_kms::grpc::kms::v1::crypto_key_version::CryptoKeyVersionAlgorithm::EcSignSecp256k1Sha256;
use google_cloud_kms::grpc::kms::v1::digest::Digest::Sha256;
use google_cloud_kms::grpc::kms::v1::{AsymmetricSignRequest, Digest, GetPublicKeyRequest};
use k256::{pkcs8::DecodePublicKey, PublicKey as K256PublicKey};

use super::signature_utils;

const GOOGLE_KMS_ERROR_PREFIX: &str = "Google KMS Error";

#[derive(Clone, Debug)]
pub struct GoogleKmsSigner {
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

impl GoogleKmsSigner {
    pub async fn new(key_path: impl Into<String>, client: &Client) -> Result<Self> {
        let key_path: String = key_path.into();
        let public_key_pem = Self::retrieve_public_key(client, &key_path).await?;
        let fuel_address = Self::derive_fuel_address(&public_key_pem)?;

        Ok(Self {
            key_path,
            client: client.clone(),
            public_key_pem,
            fuel_address,
        })
    }

    async fn retrieve_public_key(client: &Client, key_path: &str) -> Result<String> {
        let request = GetPublicKeyRequest {
            name: key_path.to_string(),
        };

        let response = client
            .get_public_key(request, None)
            .await
            .map_err(|e| format_gcp_error(format!("Failed to get public key: {}", e)))?;

        if response.algorithm != EcSignSecp256k1Sha256 as i32 {
            return Err(Error::Other(format!(
                "{GOOGLE_KMS_ERROR_PREFIX}: Invalid key algorithm: {}, expected EC_SIGN_SECP256K1_SHA256",
                response.algorithm
            )));
        }

        Ok(response.pem)
    }

    fn derive_fuel_address(pem: &str) -> Result<Bech32Address> {
        let k256_key = K256PublicKey::from_public_key_pem(pem).map_err(|_| {
            Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: Invalid PEM encoding"))
        })?;

        let fuel_public_key = PublicKey::from(k256_key);
        Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_public_key.hash()))
    }

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

    pub fn key_path(&self) -> &String {
        &self.key_path
    }

    pub fn public_key(&self) -> &String {
        &self.public_key_pem
    }
}

#[async_trait]
impl Signer for GoogleKmsSigner {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_gcp_signature(message).await?;

        let k256_key = K256PublicKey::from_public_key_pem(&self.public_key_pem).map_err(|_| {
            Error::Other(format!(
                "{GOOGLE_KMS_ERROR_PREFIX}: Invalid cached public key"
            ))
        })?;

        let (normalized_sig, recovery_id) = signature_utils::normalize_signature(
            &signature_der,
            message,
            &k256_key,
            GOOGLE_KMS_ERROR_PREFIX,
        )?;

        Ok(signature_utils::convert_to_fuel_signature(
            normalized_sig,
            recovery_id,
        ))
    }

    fn address(&self) -> &Bech32Address {
        &self.fuel_address
    }
}

fn format_gcp_error(err: impl std::fmt::Display) -> Error {
    Error::Other(format!("{GOOGLE_KMS_ERROR_PREFIX}: {err}"))
}
