#![cfg(feature = "std")]

use fuel_core_client::client::schema::message::MessageProof as ClientMessageProof;
use fuel_tx::Bytes32;
use fuel_types::Nonce;

use crate::{bech32::Bech32Address, block::Header};

#[derive(Debug)]
pub struct MerkleProof {
    /// The proof set of the message proof.
    pub proof_set: Vec<Bytes32>,
    /// The index that was used to produce this proof.
    pub proof_index: u64,
}

#[derive(Debug)]
pub struct MessageProof {
    /// Proof that message is contained within the provided block header.
    pub message_proof: MerkleProof,
    /// Proof that the provided block header is contained within the blockchain history.
    pub block_proof: MerkleProof,
    /// The previous fuel block header that contains the message. Message block height <
    /// commit block height.
    pub message_block_header: Header,
    /// The consensus header associated with the finalized commit being used
    /// as the root of the block proof.
    pub commit_block_header: Header,
    pub sender: Bech32Address,
    pub recipient: Bech32Address,
    pub nonce: Nonce,
    pub amount: u64,
    pub data: Vec<u8>,
}

impl From<ClientMessageProof> for MessageProof {
    fn from(client_message_proof: ClientMessageProof) -> Self {
        let message_proof_set = client_message_proof
            .message_proof
            .proof_set
            .iter()
            .map(|proof| proof.0 .0)
            .collect();
        let block_proof_set = client_message_proof
            .block_proof
            .proof_set
            .iter()
            .map(|proof| proof.0 .0)
            .collect();

        Self {
            message_proof: MerkleProof {
                proof_set: message_proof_set,
                proof_index: client_message_proof.message_proof.proof_index.0,
            },
            block_proof: MerkleProof {
                proof_set: block_proof_set,
                proof_index: client_message_proof.block_proof.proof_index.0,
            },
            message_block_header: client_message_proof.message_block_header.into(),
            commit_block_header: client_message_proof.commit_block_header.into(),
            sender: client_message_proof.sender.0 .0.into(),
            recipient: client_message_proof.recipient.0 .0.into(),
            nonce: client_message_proof.nonce.0 .0,
            amount: client_message_proof.amount.0,
            data: client_message_proof.data.0 .0,
        }
    }
}
