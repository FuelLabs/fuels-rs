use fuel_gql_client::client::schema::message::Message as SchemaMessage;
use fuel_tx::Address;

#[derive(Debug)]
pub struct Message {
    schema_message: SchemaMessage,
}

impl From<SchemaMessage> for Message {
    fn from(schema_message: SchemaMessage) -> Self {
        Self { schema_message }
    }
}

impl Message {
    pub fn sender(&self) -> Address {
        self.schema_message.sender.into()
    }

    pub fn recipient(&self) -> Address {
        self.schema_message.recipient.into()
    }

    pub fn nonce(&self) -> u64 {
        self.schema_message.nonce
    }

    pub fn amount(&self) -> u64 {
        self.schema_message.amount
    }

    pub fn data(&self) -> Vec<u8> {
        self.schema_message.data
    }

    pub fn da_height(&self) -> u64 {
        self.schema_message.da_height.0
    }

    pub fn fuel_block_spend(&self) -> Option<u32> {
        self.schema_message.fuel_block_spend
    }
    
}
