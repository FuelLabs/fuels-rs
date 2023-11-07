use fuel_tx::UtxoId;
use fuel_types::Nonce;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoinTypeId {
    UtxoId(UtxoId),
    Nonce(Nonce),
}
