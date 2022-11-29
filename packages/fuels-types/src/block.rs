use chrono::{DateTime, NaiveDateTime, Utc};
use fuel_gql_client::client::schema::block::{Block as SchemaBlock, Header as SchemaHeader};
use fuel_tx::Bytes32;

#[derive(Debug)]
pub struct Header<'a> {
    schema_header: &'a SchemaHeader,
}

impl<'a> Header<'a> {
    pub fn height(&self) -> u64 {
        self.schema_header.height.0
    }

    pub fn da_height(&self) -> u64 {
        self.schema_header.da_height.0
    }

    pub fn transactions_count(&self) -> u64 {
        self.schema_header.transactions_count.0
    }

    pub fn output_messages_count(&self) -> u64 {
        self.schema_header.output_messages_count.0
    }

    pub fn transactions_root(&self) -> Bytes32 {
        self.schema_header.transactions_root.0 .0
    }

    pub fn output_messages_root(&self) -> Bytes32 {
        self.schema_header.output_messages_root.0 .0
    }

    pub fn prev_root(&self) -> Bytes32 {
        self.schema_header.application_hash.0 .0
    }

    pub fn time(&self) -> Option<DateTime<Utc>> {
        let native = NaiveDateTime::from_timestamp_opt(self.schema_header.time.0 .0 as i64, 0);
        native.map(|time| DateTime::<Utc>::from_utc(time, Utc))
    }
}

#[derive(Debug)]
pub struct Block {
    schema_block: SchemaBlock,
}

impl From<SchemaBlock> for Block {
    fn from(schema_block: SchemaBlock) -> Self {
        Self { schema_block }
    }
}

impl Block {
    pub fn id(&self) -> Bytes32 {
        self.schema_block.id.0 .0
    }

    pub fn transactions(&self) -> Vec<Bytes32> {
        self.schema_block
            .transactions
            .iter()
            .map(|tx| tx.id.0 .0)
            .collect()
    }

    pub fn header(&self) -> Header {
        Header {
            schema_header: &self.schema_block.header,
        }
    }
}
