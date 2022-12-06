use crate::{coin::Coin, message::Message};
use fuel_gql_client::client::schema::resource::Resource as ClientResource;

#[derive(Debug)]
pub enum Resource {
    Coin(Coin),
    Message(Message),
}

impl From<ClientResource> for Resource {
    fn from(client_resource: ClientResource) -> Self {
        match client_resource {
            ClientResource::Coin(coin) => Resource::Coin(coin.into()),
            ClientResource::Message(message) => Resource::Message(message.into()),
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
