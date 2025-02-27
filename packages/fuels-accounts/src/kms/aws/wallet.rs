use crate::kms::aws::client::AwsClient;
use crate::provider::Provider;
use crate::wallet::Wallet;
use crate::{Account, ViewOnlyAccount};
use aws_sdk_kms::{
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
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

#[derive(Clone, Debug)]
pub struct AwsWallet {
    view_account: Wallet,
    kms_key: KmsKey,
}

#[derive(Clone, Debug)]
pub struct KmsKey {
    key_id: String,
    client: AwsClient,
    public_key: Vec<u8>,
    fuel_address: Bech32Address,
}

impl KmsKey {
    pub fn key_id(&self) -> &String {
        &self.key_id
    }
    pub fn public_key(&self) -> &Vec<u8> {
        &self.public_key
    }
    pub fn fuel_address(&self) -> &Bech32Address {
        &self.fuel_address
    }
}

impl KmsKey {
    pub async fn new(key_id: String, client: &AwsClient) -> Result<Self> {
        Self::validate_key_type(client, &key_id).await?;
        let public_key = Self::fetch_public_key(client, &key_id).await?;
        let fuel_address = Self::derive_fuel_address(&public_key)?;

        Ok(Self {
            key_id,
            client: client.clone(),
            public_key,
            fuel_address,
        })
    }

    async fn validate_key_type(client: &AwsClient, key_id: &str) -> Result<()> {
        let key_spec = client
            .inner()
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| Error::Other(format!("{}: {}", AWS_KMS_ERROR_PREFIX, e)))?
            .key_spec;

        match key_spec {
            Some(KeySpec::EccSecgP256K1) => Ok(()),
            other => Err(Error::Other(format!(
                "{}: Invalid key type {:?}, expected ECC_SECG_P256K1",
                AWS_KMS_ERROR_PREFIX, other
            ))),
        }
    }

    async fn fetch_public_key(client: &AwsClient, key_id: &str) -> Result<Vec<u8>> {
        let response = client
            .inner()
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| Error::Other(format!("{}: {}", AWS_KMS_ERROR_PREFIX, e)))?;

        response
            .public_key()
            .map(|blob| blob.as_ref().to_vec())
            .ok_or_else(|| {
                Error::Other(format!(
                    "{}: Empty public key response",
                    AWS_KMS_ERROR_PREFIX
                ))
            })
    }

    fn derive_fuel_address(public_key: &[u8]) -> Result<Bech32Address> {
        let k256_key = K256PublicKey::from_public_key_der(public_key)
            .map_err(|_| Error::Other(format!("{}: Invalid DER encoding", AWS_KMS_ERROR_PREFIX)))?;

        let fuel_public_key = PublicKey::from(k256_key);
        Ok(Bech32Address::new(FUEL_BECH32_HRP, fuel_public_key.hash()))
    }

    async fn sign_message(&self, message: Message) -> Result<Signature> {
        let signature_der = self.request_kms_signature(message).await?;
        let (sig, recovery_id) = self.normalize_signature(&signature_der, message)?;

        Ok(self.format_fuel_signature(sig, recovery_id))
    }

    async fn request_kms_signature(&self, message: Message) -> Result<Vec<u8>> {
        self.client
            .inner()
            .sign()
            .key_id(&self.key_id)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message_type(MessageType::Digest)
            .message(Blob::new(message.as_ref().to_vec()))
            .send()
            .await
            .map_err(|e| Error::Other(format!("{}: Signing failed - {}", AWS_KMS_ERROR_PREFIX, e)))?
            .signature
            .map(|blob| blob.into_inner())
            .ok_or_else(|| {
                Error::Other(format!(
                    "{}: Empty signature response",
                    AWS_KMS_ERROR_PREFIX
                ))
            })
    }

    fn normalize_signature(
        &self,
        signature_der: &[u8],
        message: Message,
    ) -> Result<(K256Signature, RecoveryId)> {
        let mut sig = K256Signature::from_der(signature_der).map_err(|_| {
            Error::Other(format!("{}: Invalid DER signature", AWS_KMS_ERROR_PREFIX))
        })?;

        sig = sig.normalize_s().unwrap_or(sig);

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

    fn format_fuel_signature(
        &self,
        signature: K256Signature,
        recovery_id: RecoveryId,
    ) -> Signature {
        let recovery_byte = recovery_id.is_y_odd() as u8;
        let mut bytes: [u8; 64] = signature.to_bytes().into();
        bytes[63] = (recovery_byte << 7) | (bytes[63] & 0x7F);
        Signature::from_bytes(bytes)
    }

    pub fn address(&self) -> &Bech32Address {
        &self.fuel_address
    }
}

impl AwsWallet {
    pub async fn with_kms_key(
        key_id: impl Into<String>,
        aws_client: &AwsClient,
        provider: Option<Provider>,
    ) -> Result<Self> {
        let kms_key = KmsKey::new(key_id.into(), aws_client).await?;

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
        self.provider().ok_or_else(|| {
            Error::Other("Provider required - use `.with_provider()` when creating wallet".into())
        })
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
