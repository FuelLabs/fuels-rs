use fuel_asm::Word;
use fuel_tx::{Address, AssetId, Input as FuelInput, TxPointer, UtxoId};
use fuel_types::{Bytes32, ContractId, MessageId};

use crate::unresolved_bytes::UnresolvedBytes;
use crate::{coin::Coin, message::Message};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    CoinSigned(Coin),
    MessageSigned(Message),
    CoinPredicate {
        coin: Coin,
        code: Vec<u8>,
        data: UnresolvedBytes,
    },
    MessagePredicate {
        message: Message,
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
    pub const fn coin_predicate(coin: Coin, code: Vec<u8>, data: UnresolvedBytes) -> Self {
        Self::CoinPredicate { coin, code, data }
    }

    pub const fn message_predicate(message: Message, code: Vec<u8>, data: UnresolvedBytes) -> Self {
        Self::MessagePredicate {
            message,
            code,
            data,
        }
    }
}

impl From<Input> for FuelInput {
    fn from(input: Input) -> Self {
        match input {
            Input::CoinSigned(coin) => todo!(),
            Input::MessageSigned(_) => todo!(),
            Input::CoinPredicate { coin, code, data } => todo!(),
            Input::MessagePredicate {
                message,
                code,
                data,
            } => todo!(),
            Input::Contract { .. } => todo!(),
        }
    }
}
