use crate::{coin::Coin, message::Message};
use fuel_gql_client::client::schema::resource::Resource as ClientResource;

#[derive(Debug)]
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
}
