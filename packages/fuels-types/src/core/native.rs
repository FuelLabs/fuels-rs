use fuel_tx::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Parameterize, Tokenizable)]
#[FuelsTypesPath = "crate"]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}
