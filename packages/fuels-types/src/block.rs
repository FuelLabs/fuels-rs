use chrono::{DateTime, NaiveDateTime, Utc};
use fuel_gql_client::client::schema::block::{Block as ClientBlock, Header as ClientHeader};
use fuel_tx::Bytes32;

#[derive(Debug)]
pub struct Header {
    pub id: Bytes32,
    pub da_height: u64,
    pub transactions_count: u64,
    pub output_messages_count: u64,
    pub transactions_root: Bytes32,
    pub output_messages_root: Bytes32,
    pub height: u64,
    pub prev_root: Bytes32,
    pub time: Option<DateTime<Utc>>,
    pub application_hash: Bytes32,
}

impl From<ClientHeader> for Header {
    fn from(client_header: ClientHeader) -> Self {
        let naive = NaiveDateTime::from_timestamp_opt(client_header.time.0.to_unix(), 0);
        let time = naive.map(|time| DateTime::<Utc>::from_utc(time, Utc));

        Self {
            id: client_header.id.0 .0,
            da_height: client_header.da_height.0,
            transactions_count: client_header.transactions_count.0,
            output_messages_count: client_header.output_messages_count.0,
            transactions_root: client_header.transactions_root.0 .0,
            output_messages_root: client_header.output_messages_root.0 .0,
            height: client_header.height.0,
            prev_root: client_header.prev_root.0 .0,
            time,
            application_hash: client_header.application_hash.0 .0,
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub id: Bytes32,
    pub header: Header,
    pub transactions: Vec<Bytes32>,
}

impl From<ClientBlock> for Block {
    fn from(client_block: ClientBlock) -> Self {
        let transactions = client_block
            .transactions
            .iter()
            .map(|tx| tx.id.0 .0)
            .collect();

        Self {
            id: client_block.id.0 .0,
            header: client_block.header.into(),
            transactions,
        }
    }
}
