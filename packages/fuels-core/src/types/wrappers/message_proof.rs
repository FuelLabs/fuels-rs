#![cfg(feature = "std")]

use fuel_core_client::client::types::{
    primitives::Nonce, MerkleProof as ClientMerkleProof, MessageProof as ClientMessageProof,
};
use fuel_types::Bytes32;

use crate::types::{bech32::Bech32Address, block::Header};

#[derive(Debug)]
pub struct MerkleProof {
    /// The proof set of the message proof.
    pub proof_set: Vec<Bytes32>,
    /// The index that was used to produce this proof.
    pub proof_index: u64,
}

impl From<ClientMerkleProof> for MerkleProof {
    fn from(client_merkle_proof: ClientMerkleProof) -> Self {
        Self {
            proof_set: client_merkle_proof.proof_set,
            proof_index: client_merkle_proof.proof_index,
        }
    }
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
        Self {
            message_proof: client_message_proof.message_proof.into(),
            block_proof: client_message_proof.block_proof.into(),
            message_block_header: client_message_proof.message_block_header.into(),
            commit_block_header: client_message_proof.commit_block_header.into(),
            sender: client_message_proof.sender.into(),
            recipient: client_message_proof.recipient.into(),
            nonce: client_message_proof.nonce,
            amount: client_message_proof.amount,
            data: client_message_proof.data,
        }
    }
}
