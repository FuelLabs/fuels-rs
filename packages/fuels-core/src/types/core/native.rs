use fuel_tx::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable, TryFrom};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Parameterize, Tokenizable, TryFrom,
)]
#[FuelsCorePath = "crate"]
#[FuelsTypesPath = "crate::types"]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}

impl Default for Identity {
    fn default() -> Self {
        Self::Address(Address::default())
    }
}
