use fuel_asm::Word;
use fuel_tx::{Address, AssetId, Input as FuelInput, TxPointer, UtxoId};
use fuel_types::{Bytes32, ContractId, MessageId};

use crate::resource::Resource;
use crate::unresolved_bytes::UnresolvedBytes;
use crate::{coin::Coin, message::Message};

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
            Input::ResourceSigned { resource, ..}
            | Input::ResourcePredicate { resource, ..} => Some(resource.amount()),
            _ => None,
        }
    }
}
