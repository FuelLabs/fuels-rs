use fuel_gql_client::client::schema::message::MessageProof as SchemaMessageProof;
use fuel_tx::{Address, Bytes32, Bytes64};

use crate::block::Header;

#[derive(Debug)]
pub struct MessageProof {
    schema_message_proof: SchemaMessageProof,
}

impl From<SchemaMessageProof> for MessageProof {
    fn from(schema_message_proof: SchemaMessageProof) -> Self {
        Self {
            schema_message_proof,
        }
    }
}

impl MessageProof {
    pub fn proof_set(&self) -> Vec<Bytes32> {
        self.schema_message_proof
            .proof_set
            .iter()
            .map(|proof| proof.0 .0)
            .collect()
    }

    pub fn proof_index(&self) -> u64 {
        self.schema_message_proof.proof_index.0
    }

    pub fn signature(&self) -> Bytes64 {
        self.schema_message_proof.signature.0 .0
    }

    pub fn header(&self) -> Header {
        Header {
            schema_header: &self.schema_message_proof.header,
        }
    }

    pub fn sender(&self) -> Address {
        self.schema_message_proof.sender.0 .0
    }

    pub fn recipient(&self) -> Address {
        self.schema_message_proof.recipient.0 .0
    }

    pub fn nonce(&self) -> Bytes32 {
        Bytes32::from(*self.schema_message_proof.recipient.0 .0)
    }

    pub fn amount(&self) -> u64 {
        self.schema_message_proof.amount.0
    }

    pub fn data(&self) -> &[u8] {
        &self.schema_message_proof.data.0 .0
    }
}
