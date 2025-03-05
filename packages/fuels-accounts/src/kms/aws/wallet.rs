use crate::accounts_utils::try_provider_error;
use crate::provider::Provider;
use crate::wallet::Wallet;
use crate::{Account, ViewOnlyAccount};
use aws_sdk_kms::{
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
    Client,
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

const AWS_KMS_ERROR_PREFIX: &str = "AWS KMS Error";
const EXPECTED_KEY_SPEC: KeySpec = KeySpec::EccSecgP256K1;

/// A wallet implementation that uses AWS KMS for signing
#[derive(Clone, Debug)]
pub struct AwsWallet {
    view_account: Wallet,
    kms_key: KmsKey,
}

#[derive(Clone, Debug)]
pub struct KmsKey {
    key_id: String,
    client: Client,
    public_key_der: Vec<u8>,
    fuel_address: Bech32Address,
}

impl KmsKey {
    pub fn key_id(&self) -> &String {
        &self.key_id
    }

    pub fn public_key(&self) -> &Vec<u8> {
        &self.public_key_der
    }

    pub fn fuel_address(&self) -> &Bech32Address {
        &self.fuel_address
    }

    /// Creates a new KmsKey from an AWS KMS key ID
    pub async fn new(key_id: String, client: &Client) -> Result<Self> {
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

    /// Validates that the KMS key is of the expected type
    async fn validate_key_spec(client: &Client, key_id: &str) -> Result<()> {
        let response = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(format_kms_error)?;

        let key_spec = response.key_spec;

        match key_spec {
            Some(EXPECTED_KEY_SPEC) => Ok(()),
            other => Err(Error::Other(format!(
                "{AWS_KMS_ERROR_PREFIX}: Invalid key type {other:?}, expected {EXPECTED_KEY_SPEC:?}"
            ))),
        }
    }

    /// Retrieves the public key from AWS KMS
    async fn retrieve_public_key(client: &Client, key_id: &str) -> Result<Vec<u8>> {
        let response = client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(format_kms_error)?;

        response
            .public_key()
            .map(|blob| blob.as_ref().to_vec())
            .ok_or_else(|| {
                Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Empty public key response"))
            })
    }

    /// Derives a Fuel address from a public key in DER format
    fn derive_fuel_address(public_key: &[u8]) -> Result<Bech32Address> {
        let k256_key = K256PublicKey::from_public_key_der(public_key)
            .map_err(|_| Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Invalid DER encoding")))?;

        let fuel_public_key = PublicKey::from(k256_key);
        Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_public_key.hash()))
    }

    /// Signs a message using the AWS KMS key
    async fn sign_message(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_kms_signature(message).await?;
        let (sig, recovery_id) = self.normalize_signature(&signature_der, message)?;

        Ok(self.convert_to_fuel_signature(sig, recovery_id))
    }

    /// Requests a signature from AWS KMS
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

    /// Normalizes a DER signature and determines the recovery ID
    fn normalize_signature(
        &self,
        signature_der: &[u8],
        message: Message,
    ) -> Result<(K256Signature, RecoveryId)> {
        let signature = K256Signature::from_der(signature_der)
            .map_err(|_| Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Invalid DER signature")))?;

        // Ensure the signature is in normalized form (low-S value)
        let normalized_sig = signature.normalize_s().unwrap_or(signature);
        let recovery_id = self.determine_recovery_id(&normalized_sig, message)?;

        Ok((normalized_sig, recovery_id))
    }

    /// Determines the correct recovery ID for the signature
    fn determine_recovery_id(&self, sig: &K256Signature, message: Message) -> Result<RecoveryId> {
        let recid_even = RecoveryId::new(false, false);
        let recid_odd = RecoveryId::new(true, false);

        // Get the expected public key
        let expected_pubkey = K256PublicKey::from_public_key_der(&self.public_key_der)
            .map_err(|_| {
                Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: Invalid cached public key"))
            })?
            .into();

        // Try recovery with each recovery ID
        let recovered_even = VerifyingKey::recover_from_prehash(&*message, sig, recid_even);
        let recovered_odd = VerifyingKey::recover_from_prehash(&*message, sig, recid_odd);

        if recovered_even
            .map(|r| r == expected_pubkey)
            .unwrap_or(false)
        {
            Ok(recid_even)
        } else if recovered_odd.map(|r| r == expected_pubkey).unwrap_or(false) {
            Ok(recid_odd)
        } else {
            Err(Error::Other(format!(
                "{AWS_KMS_ERROR_PREFIX}: Invalid signature (could not recover correct public key)"
            )))
        }
    }

    /// Converts a k256 signature to a Fuel signature format
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

    pub fn address(&self) -> &Bech32Address {
        &self.fuel_address
    }
}

impl AwsWallet {
    /// Creates a new AwsWallet with the given KMS key ID
    pub async fn with_kms_key(
        key_id: impl Into<String>,
        aws_client: &Client,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let kms_key = KmsKey::new(key_id.into(), aws_client).await?;

        Ok(Self {
            view_account: Wallet::from_address(kms_key.fuel_address.clone(), provider),
            kms_key,
        })
    }

    /// Returns the Fuel address associated with this wallet
    pub fn address(&self) -> &Bech32Address {
        &self.kms_key.fuel_address
    }

    /// Returns the provider associated with this wallet, if any
    pub fn provider(&self) -> Option<&Provider> {
        self.view_account.provider()
    }
}

#[async_trait::async_trait]
impl Signer for AwsWallet {
    async fn sign(&self, message: Message) -> Result<Signature> {
        self.kms_key.sign_message(message).await
    }

    fn address(&self) -> &Bech32Address {
        self.address()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ViewOnlyAccount for AwsWallet {
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
impl Account for AwsWallet {
    fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
        tb.add_signer(self.clone())?;
        Ok(())
    }
}

fn format_kms_error(err: impl std::fmt::Display) -> Error {
    Error::Other(format!("{AWS_KMS_ERROR_PREFIX}: {err}"))
}
