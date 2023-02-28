use fuel_asm::Word;
use fuel_tx::{Address, AssetId, Input as FuelInput, TxPointer, UtxoId};

use crate::{coin::Coin, message::Message, unresolved_bytes::UnresolvedBytes};

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
    Contract(fuel_tx::Input),
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
            Input::Contract(_) => todo!(),
        }
    }
}

impl Input {}
