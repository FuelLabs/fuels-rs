use aws_sdk_kms::{
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
    // Client as AwsClient,
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
use crate::aws::{AwsClient, AwsConfig};

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
    pub public_key: Vec<u8>,
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
            .inner()
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
        let response = client.inner().get_public_key().key_id(key_id).send().await?;

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
            .inner()
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
        let config = AwsConfig::from_env().await;
        let client = AwsClient::new(config);
        let kms_data = KmsData::new(kms_key_id, client).await?;

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