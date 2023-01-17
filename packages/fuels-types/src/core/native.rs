use fuel_tx::{Address, ContractId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}
