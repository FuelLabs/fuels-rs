#![cfg(feature = "std")]

use chrono::{DateTime, NaiveDateTime, Utc};
use fuel_core_client::client::types::block::{Block as ClientBlock, Header as ClientHeader};
use fuel_core_client::client::types::primitives::Bytes32;

#[derive(Debug)]
pub struct Header {
    pub id: Bytes32,
    pub da_height: u64,
    pub transactions_count: u64,
    pub message_receipt_count: u64,
    pub transactions_root: Bytes32,
    pub message_receipt_root: Bytes32,
    pub height: u32,
    pub prev_root: Bytes32,
    pub time: Option<DateTime<Utc>>,
    pub application_hash: Bytes32,
}

impl From<ClientHeader> for Header {
    fn from(client_header: ClientHeader) -> Self {
        let naive = NaiveDateTime::from_timestamp_opt(client_header.time.to_unix(), 0);
        let time = naive.map(|time| DateTime::<Utc>::from_utc(time, Utc));

        Self {
            id: client_header.id.into(),
            da_height: client_header.da_height.into(),
            transactions_count: client_header.transactions_count.into(),
            message_receipt_count: client_header.message_receipt_count.into(),
            transactions_root: client_header.transactions_root.into(),
            message_receipt_root: client_header.message_receipt_root.into(),
            height: client_header.height.into(),
            prev_root: client_header.prev_root.into(),
            time,
            application_hash: client_header.application_hash.into(),
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
        let transactions = client_block.transactions.into_iter().collect();

        Self {
            id: client_block.id.into(),
            header: client_block.header.into(),
            transactions,
        }
    }
}
