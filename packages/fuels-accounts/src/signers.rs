use async_trait::async_trait;
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuels_core::{
    traits::Signer,
    types::{
        bech32::{Bech32Address, FUEL_BECH32_HRP},
        errors::Result,
    },
};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKeySigner {
    private_key: SecretKey,
    #[zeroize(skip)]
    address: Bech32Address,
}

impl PrivateKeySigner {
    pub fn new(private_key: SecretKey) -> Self {
        let public = PublicKey::from(&private_key);
        let hashed = public.hash();
        let address = Bech32Address::new(FUEL_BECH32_HRP, hashed);

        Self {
            private_key,
            address,
        }
    }
}

#[async_trait]
impl Signer for PrivateKeySigner {
    async fn sign(&self, message: Message) -> Result<Signature> {
        let sig = Signature::sign(&self.private_key, &message);

        Ok(sig)
    }

    fn address(&self) -> &Bech32Address {
        &self.address
    }
}
