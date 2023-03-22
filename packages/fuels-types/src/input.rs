#![cfg(feature = "std")]

use std::hash::Hash;

use fuel_tx::{TxPointer, UtxoId};
use fuel_types::{Bytes32, ContractId};

use crate::{resource::Resource, unresolved_bytes::UnresolvedBytes};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Input {
    ResourceSigned {
        resource: Resource,
        witness_index: u8,
    },
    ResourcePredicate {
        resource: Resource,
        code: Vec<u8>,
        data: UnresolvedBytes,
    },
    Contract {
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    },
}

impl Input {
    pub const fn resource_signed(resource: Resource, witness_index: u8) -> Self {
        Self::ResourceSigned {
            resource,
            witness_index,
        }
    }

    pub const fn resource_predicate(
        resource: Resource,
        code: Vec<u8>,
        data: UnresolvedBytes,
    ) -> Self {
        Self::ResourcePredicate {
            resource,
            code,
            data,
        }
    }

    pub fn amount(&self) -> Option<u64> {
        match self {
            Self::ResourceSigned { resource, .. } | Self::ResourcePredicate { resource, .. } => {
                Some(resource.amount())
            }
            _ => None,
        }
    }

    pub const fn contract(
        utxo_id: UtxoId,
        balance_root: Bytes32,
        state_root: Bytes32,
        tx_pointer: TxPointer,
        contract_id: ContractId,
    ) -> Self {
        Self::Contract {
            utxo_id,
            balance_root,
            state_root,
            tx_pointer,
            contract_id,
        }
    }
}
