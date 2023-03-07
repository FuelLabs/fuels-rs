#![cfg(feature = "std")]

use fuel_core_client::client::schema::resource::Resource as ClientResource;
use fuel_tx::AssetId;

use crate::{coin::Coin, constants::BASE_ASSET_ID, message::Message};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resource {
    Coin(Coin),
    Message(Message),
}

impl TryFrom<ClientResource> for Resource {
    type Error = std::io::Error;

    fn try_from(client_resource: ClientResource) -> Result<Self, Self::Error> {
        match client_resource {
            ClientResource::Coin(coin) => Ok(Resource::Coin(coin.into())),
            ClientResource::Message(message) => Ok(Resource::Message(message.into())),
            ClientResource::Unknown => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Got unknown `ClientResource`",
            )),
        }
    }
}

impl Resource {
    pub fn amount(&self) -> u64 {
        match self {
            Resource::Coin(coin) => coin.amount,
            Resource::Message(message) => message.amount,
        }
    }

    pub fn asset_id(&self) -> AssetId {
        match self {
            Resource::Coin(coin) => coin.asset_id,
            Resource::Message(_) => BASE_ASSET_ID,
        }
    }
}
