use fuel_types::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable, TryFrom};
use serde::{Deserialize, Serialize};

use crate::types::bech32::{Bech32Address, Bech32ContractId};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Parameterize, Tokenizable, TryFrom, Serialize, Deserialize,
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

impl AsRef<[u8]> for Identity {
    fn as_ref(&self) -> &[u8] {
        match self {
            Identity::Address(address) => address.as_ref(),
            Identity::ContractId(contract_id) => contract_id.as_ref(),
        }
    }
}

impl From<&Address> for Identity {
    fn from(address: &Address) -> Self {
        Self::Address(*address)
    }
}
impl From<Address> for Identity {
    fn from(address: Address) -> Self {
        Self::Address(address)
    }
}

impl From<&ContractId> for Identity {
    fn from(contract_id: &ContractId) -> Self {
        Self::ContractId(*contract_id)
    }
}
impl From<ContractId> for Identity {
    fn from(contract_id: ContractId) -> Self {
        Self::ContractId(contract_id)
    }
}

impl From<&Bech32Address> for Identity {
    fn from(data: &Bech32Address) -> Self {
        Self::Address(data.into())
    }
}
impl From<Bech32Address> for Identity {
    fn from(data: Bech32Address) -> Self {
        Self::Address(data.into())
    }
}

impl From<&Bech32ContractId> for Identity {
    fn from(data: &Bech32ContractId) -> Self {
        Self::ContractId(data.into())
    }
}
impl From<Bech32ContractId> for Identity {
    fn from(data: Bech32ContractId) -> Self {
        Self::ContractId(data.into())
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_bech32() {
        let b32_str = "fuel1dved7k25uxadatl7l5kql309jnw07dcn4t3a6x9hm9nxyjcpqqns50p7n2";
        let bech32_contract_id = Bech32ContractId::from_str(b32_str).unwrap();

        let bech32_contract_id_borrowed = &bech32_contract_id.clone();
        let identity: Identity = bech32_contract_id_borrowed.into();
        assert_eq!(
            identity,
            Identity::ContractId(bech32_contract_id.clone().into())
        );

        let identity: Identity = bech32_contract_id.clone().into();
        assert_eq!(identity, Identity::ContractId(bech32_contract_id.into()));

        let bech32_address = Bech32Address::from_str(b32_str).unwrap();

        let bech32_address_borrowed = &bech32_address.clone();
        let identity: Identity = bech32_address_borrowed.into();
        assert_eq!(identity, Identity::Address(bech32_address.clone().into()));

        let identity: Identity = bech32_address.clone().into();
        assert_eq!(identity, Identity::Address(bech32_address.clone().into()));
    }
}
