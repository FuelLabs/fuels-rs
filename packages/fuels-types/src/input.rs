use std::hash::{Hash, Hasher};
use fuel_tx::{TxPointer, UtxoId};
use fuel_types::{Bytes32, ContractId};

use crate::resource::Resource;
use crate::unresolved_bytes::UnresolvedBytes;

#[derive(Debug, Clone, PartialEq, Eq)]
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


impl Hash for Input {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Input::ResourceSigned {
                resource, ..
            } => {
                0.hash(state);
                resource.hash(state);
            }
            Input::ResourcePredicate {
                resource, ..
            } => {
                1.hash(state);
                resource.hash(state);
            }
            Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                tx_pointer,
                contract_id,
            } => {
                2.hash(state);
                utxo_id.hash(state);
                balance_root.hash(state);
                state_root.hash(state);
                tx_pointer.hash(state);
                contract_id.hash(state);
            }
        }
    }
}