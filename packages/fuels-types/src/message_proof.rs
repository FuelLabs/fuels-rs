use fuel_core_client::client::schema::message::MessageProof as ClientMessageProof;
use fuel_tx::{Bytes32, Bytes64};

use crate::{bech32::Bech32Address, block::Header};

#[derive(Debug)]
pub struct MessageProof {
    pub proof_set: Vec<Bytes32>,
    pub proof_index: u64,
    pub signature: Bytes64,
    pub header: Header,
    pub sender: Bech32Address,
    pub recipient: Bech32Address,
    pub nonce: Bytes32,
    pub amount: u64,
    pub data: Vec<u8>,
}

impl From<ClientMessageProof> for MessageProof {
    fn from(client_message_proof: ClientMessageProof) -> Self {
        let proof_set = client_message_proof
            .proof_set
            .iter()
            .map(|proof| proof.0 .0)
            .collect();

        Self {
            proof_set,
            proof_index: client_message_proof.proof_index.0,
            signature: client_message_proof.signature.0 .0,
            header: client_message_proof.header.into(),
            sender: client_message_proof.sender.0 .0.into(),
            recipient: client_message_proof.recipient.0 .0.into(),
            nonce: client_message_proof.nonce.0 .0,
            amount: client_message_proof.amount.0,
            data: client_message_proof.data.0 .0,
        }
    }
}
