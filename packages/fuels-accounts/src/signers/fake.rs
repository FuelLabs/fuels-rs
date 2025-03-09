use async_trait::async_trait;
use fuel_crypto::{Message, Signature};
use fuels_core::{
    traits::Signer,
    types::{bech32::Bech32Address, errors::Result},
};

use super::private_key::PrivateKeySigner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeSigner {
    address: Bech32Address,
}

impl From<PrivateKeySigner> for FakeSigner {
    fn from(signer: PrivateKeySigner) -> Self {
        Self {
            address: signer.address().clone(),
        }
    }
}

impl FakeSigner {
    pub fn new(address: Bech32Address) -> Self {
        Self { address }
    }
}

#[async_trait]
impl Signer for FakeSigner {
    async fn sign(&self, _message: Message) -> Result<Signature> {
        Ok(Signature::default())
    }

    fn address(&self) -> &Bech32Address {
        &self.address
    }
}
