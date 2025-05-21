use async_trait::async_trait;
use fuel_crypto::{Message, Signature};
use fuels_core::{
    traits::Signer,
    types::{Address, errors::Result},
};

use super::private_key::PrivateKeySigner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeSigner {
    address: Address,
}

impl From<PrivateKeySigner> for FakeSigner {
    fn from(signer: PrivateKeySigner) -> Self {
        Self {
            address: signer.address(),
        }
    }
}

impl FakeSigner {
    pub fn new(address: Address) -> Self {
        Self { address }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for FakeSigner {
    async fn sign(&self, _message: Message) -> Result<Signature> {
        Ok(Signature::default())
    }

    fn address(&self) -> Address {
        self.address
    }
}
