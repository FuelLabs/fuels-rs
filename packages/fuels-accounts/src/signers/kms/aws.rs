use async_trait::async_trait;
pub use aws_config;
pub use aws_sdk_kms;
use aws_sdk_kms::{
    Client,
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
};
use fuel_crypto::{Message, PublicKey, Signature};
use fuels_core::{
    traits::Signer,
    types::{
        Address,
        errors::{Error, Result},
    },
};
use k256::{PublicKey as K256PublicKey, pkcs8::DecodePublicKey};

use super::signature_utils;

const AWS_KMS_ERROR_PREFIX: &str = "AWS KMS Error";

const EXPECTED_KEY_SPEC: KeySpec = KeySpec::EccSecgP256K1;

#[derive(Clone, Debug)]
pub struct AwsKmsSigner {
    key_id: String,
    client: Client,
    public_key_der: Vec<u8>,
    fuel_address: Address,
}

impl AwsKmsSigner {
    pub async fn new(key_id: impl Into<String>, client: &Client) -> Result<Self> {
        let key_id: String = key_id.into();
        Self::validate_key_spec(client, &key_id).await?;
        let public_key = Self::retrieve_public_key(client, &key_id).await?;
        let fuel_address = Self::derive_fuel_address(&public_key)?;

        Ok(Self {
            key_id,
            client: client.clone(),
            public_key_der: public_key,
            fuel_address,
        })
    }

    async fn validate_key_spec(client: &Client, key_id: &str) -> Result<()> {
        let response = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(format_aws_error)?;

        let key_spec = response.key_spec;

        match key_spec {
            Some(EXPECTED_KEY_SPEC) => Ok(()),
            other => Err(Error::Other(format!(
                "{AWS_KMS_ERROR_PREFIX}: Invalid key type {other:?}, expected {EXPECTED_KEY_SPEC:?}"
            ))),
        }
    }

    async fn retrieve_public_key(client: &Client, key_id: &str) -> Result<Vec<u8>> {
        let response = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(format_aws_error)?;

        response
            .public_key()
            .map(|blob| blob.as_ref().to_vec())
            .ok_or_else(|| {
                Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Empty public key response"))
            })
    }

    fn derive_fuel_address(public_key: &[u8]) -> Result<Address> {
        let k256_key = K256PublicKey::from_public_key_der(public_key)
            .map_err(|_| Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Invalid DER encoding")))?;

        let fuel_public_key = PublicKey::from(k256_key);

        Ok(Address::from(*fuel_public_key.hash()))
    }

    async fn request_kms_signature(&self, message: Message) -> Result<Vec<u8>> {
        let response = self
            .client
            .sign()
            .key_id(&self.key_id)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message_type(MessageType::Digest)
            .message(Blob::new(message.as_ref().to_vec()))
            .send()
            .await
            .map_err(|err| {
                Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Signing failed - {err}"))
            })?;

        response
            .signature
            .map(|blob| blob.into_inner())
            .ok_or_else(|| {
                Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Empty signature response"))
            })
    }

    pub fn key_id(&self) -> &String {
        &self.key_id
    }

    pub fn public_key(&self) -> &Vec<u8> {
        &self.public_key_der
    }
}

#[async_trait]
impl Signer for AwsKmsSigner {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_kms_signature(message).await?;

        let k256_key = K256PublicKey::from_public_key_der(&self.public_key_der).map_err(|_| {
            Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Invalid cached public key"))
        })?;

        let (normalized_sig, recovery_id) = signature_utils::normalize_signature(
            &signature_der,
            message,
            &k256_key,
            AWS_KMS_ERROR_PREFIX,
        )?;

        Ok(signature_utils::convert_to_fuel_signature(
            normalized_sig,
            recovery_id,
        ))
    }

    fn address(&self) -> Address {
        self.fuel_address
    }
}

fn format_aws_error(err: impl std::fmt::Display) -> Error {
    Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: {err}"))
}
