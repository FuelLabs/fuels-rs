use crate::errors::Error;
use bech32::Variant::Bech32m;
use bech32::{FromBase32, ToBase32};
use core::str;
use std::str::FromStr;
use fuel_tx::{Address, Bytes32, ContractId};
use std::fmt::{Display, Formatter};

pub const FUEL_BECH32_HRP: &str = "fuel";

/// Bech32 data consisting of a human-readable part (hrp)
/// and a public key- or contract hash
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bech32Data {
    pub hrp: String,
    pub hash: Bytes32,
}

impl Bech32Data {
    pub fn new(hrp: &str, hash: Bytes32) -> Bech32Data {
        Self {
            hrp: hrp.to_string(),
            hash
        }
    }
}

impl From<Address> for Bech32Data {
    fn from(address: Address) -> Bech32Data {
        Bech32Data { hrp: FUEL_BECH32_HRP.to_string(), hash: Bytes32::new(*address) }
    }
}

impl From<ContractId> for Bech32Data {
    fn from(contract_id: ContractId) -> Bech32Data {
        Bech32Data { hrp: FUEL_BECH32_HRP.to_string(), hash: Bytes32::new(*contract_id) }
    }
}

impl From<&Bech32Data> for Address {
    fn from(data: &Bech32Data) -> Address {
        Address::new(*data.hash)
    }
}

impl From<&Bech32Data> for ContractId {
    fn from(data: &Bech32Data) -> ContractId {
        ContractId::new(*data.hash)
    }
}

impl From<Bech32Data> for Address {
    fn from(data: Bech32Data) -> Address {
        Address::new(*data.hash)
    }
}

impl From<Bech32Data> for ContractId {
    fn from(data: Bech32Data) -> ContractId {
        ContractId::new(*data.hash)
    }
}

impl FromStr for Bech32Data {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (hrp, pubkey_hash_base32, _) = bech32::decode(s)?;

        let pubkey_hash: [u8; Address::LEN] = Vec::<u8>::from_base32(&pubkey_hash_base32)?
            .as_slice()
            .try_into()?;

        Ok(Self {
            hrp,
            hash: Bytes32::from(pubkey_hash),
        })
    }
}

impl Display for Bech32Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let data_base32 = self.hash.to_base32();
        let encoding = bech32::encode(&self.hrp, &data_base32, Bech32m).unwrap();

        write!(f, "{}", encoding)
    }
}

/// An address or contract id represented by the bech32 format
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Bech32 {
    Address(Bech32Data),
    ContractId(Bech32Data),
}

impl Bech32 {
    pub fn address(hrp: &str, hash: [u8; 32]) -> Self {
        let data = Bech32Data::new(hrp, Bytes32::new(hash));

        Self::Address(data)
    }

    pub fn contract_id(hrp: &str, hash: [u8; 32]) -> Self {
        let data = Bech32Data::new(hrp, Bytes32::new(hash));

        Self::ContractId(data)
    }

    pub fn address_from_str(s: &str) -> Result<Self, Error> {
        let data = Bech32Data::from_str(s)?;

        Ok(Self::Address(data))
    }

    pub fn contract_id_from_str(s: &str) -> Result<Self, Error> {
        let data = Bech32Data::from_str(s)?;

        Ok(Self::ContractId(data))
    }

    pub fn hash(&self) -> Bytes32 {
        match self {
            Self::Address(data) => data.hash,
            Self::ContractId(data) => data.hash,
        }
    }

    pub fn hrp(&self) -> &str {
        match self {
            Self::Address(data) => &data.hrp,
            Self::ContractId(data) => &data.hrp,
        }
    }

}

impl Display for Bech32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Address(data) => data.fmt(f),
            Self::ContractId(data) => data.fmt(f),
        }
    }
}

impl From<Address> for Bech32 {
    fn from(address: Address) -> Bech32 {
        Bech32::Address(address.into())
    }
}

impl From<ContractId> for Bech32 {
    fn from(contract_id: ContractId) -> Bech32 {
        Bech32::Address(contract_id.into())
    }
}

impl From<&Bech32> for Address {
    fn from(data: &Bech32) -> Address {
        match data {
            Bech32::Address(bech32data) => bech32data.into(),
            Bech32::ContractId(bech32data) => bech32data.into(),
        }
    }
}

impl From<&Bech32> for ContractId {
    fn from(data: &Bech32) -> ContractId {
        match data {
            Bech32::Address(bech32data) => bech32data.into(),
            Bech32::ContractId(bech32data) => bech32data.into(),
        }
    }
}

impl From<Bech32> for Address {
    fn from(data: Bech32) -> Address {
        match data {
            Bech32::Address(bech32data) => bech32data.into(),
            Bech32::ContractId(bech32data) => bech32data.into(),
        }
    }
}

impl From<Bech32> for ContractId {
    fn from(data: Bech32) -> ContractId {
        match data {
            Bech32::Address(bech32data) => bech32data.into(),
            Bech32::ContractId(bech32data) => bech32data.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_new() {
        let pubkey_hash = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];
        let expected_address = Address::new(pubkey_hash);

        let bech32_addr = &Bech32::address(FUEL_BECH32_HRP, pubkey_hash);
        let plain_addr: Address = bech32_addr.into();

        assert_eq!(plain_addr, expected_address);
    }

    #[tokio::test]
    async fn test_from_str() {
        let pubkey_hash = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];

        let bech32_contract_id = &Bech32::contract_id_from_str(
            "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7",
        )
        .unwrap();
        let plain_contract_id: ContractId = bech32_contract_id.into();

        let expected_contract_id = ContractId::new(pubkey_hash);

        assert_eq!(plain_contract_id, expected_contract_id);
    }
}
