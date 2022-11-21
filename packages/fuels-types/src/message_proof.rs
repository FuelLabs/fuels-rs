use fuel_gql_client::client::schema::message::MessageProof as SchemaMessageProof;
use fuel_tx::{Bytes32, Bytes64, Address};

#[derive(Debug)]
pub struct MessageProof {
    schema_message_proof: SchemaMessageProof,
}

impl From<SchemaMessageProof> for MessageProof {
    fn from(schema_message_proof: SchemaMessageProof) -> Self {
        Self { schema_message_proof }
    }
}

impl MessageProof {
    pub fn proof_set(&self) -> Vec<Bytes32> {
        self.schema_message_proof.proof_set
    }

    pub fn proof_index(&self) -> u64 {
        self.schema_message_proof.proof_index.0
    }

    pub fn signature(&self) -> Bytes64 {
        self.schema_message_proof.signature.into()
    }

    /*
    pub fn header(&self) -> Header {
        self.schema_message_proof.header.into()
    }
    */

    pub fn sender(&self) -> Address {
        self.schema_message_proof.sender.into()
    }

    pub fn recipient(&self) -> Address {
        self.schema_message_proof.recipient.into()
    }

    pub fn nonce(&self) -> Bytes32 {
        self.schema_message_proof.recipient.into()
    }

    pub fn amount(&self) -> u64 {
        self.schema_message_proof.amount.0
    }

    pub fn data(&self) -> Vec<u8> {
        self.schema_message_proof.data.0
    }

}
