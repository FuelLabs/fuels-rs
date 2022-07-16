use crate::errors::Error;
use bech32::Variant::Bech32m;
use bech32::{FromBase32, ToBase32};
use core::str;
use fuel_tx::{Address, Bytes32, ContractId};
use std::fmt::{Display, Formatter};

pub const FUEL_BECH32_HRP: &str = "fuel";

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ContentType {
    Address,
    ContractId,
}

/// An address in the bech32 format consisting of a
/// human-readable part (HRP), '1' as a separator, the
///  public key hash encoded with a u5 charset, and a checksum.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bech32 {
    /// Public key hash or contract hash
    hash: Bytes32,

    /// String representation of the bech32 format
    encoding: String,

    pub content_type: ContentType,
}

impl Bech32 {
    pub fn new_address(hrp: &str, pubkey_hash: [u8; 32]) -> Bech32 {
        Self::new(hrp, pubkey_hash, ContentType::Address)
    }

    pub fn new_contract_id(hrp: &str, contract_hash: [u8; 32]) -> Bech32 {
        Self::new(hrp, contract_hash, ContentType::ContractId)
    }

    fn new(hrp: &str, data: [u8; 32], content_type: ContentType) -> Self {
        let data_base32 = data.to_base32();
        let encoding = bech32::encode(hrp, &data_base32, Bech32m).unwrap();

        Self {
            hash: Bytes32::from(data),
            encoding,
            content_type,
        }
    }

    pub fn to_address(&self) -> Address {
        Address::new(*self.hash)
    }

    pub fn to_contract_id(&self) -> ContractId {
        ContractId::new(*self.hash)
    }

    pub fn new_address_from_string(s: &str) -> Result<Self, Error> {
        Self::from_string(s, ContentType::Address)
    }

    pub fn new_contract_id_from_string(s: &str) -> Result<Self, Error> {
        Self::from_string(s, ContentType::ContractId)
    }

    fn from_string(s: &str, content_type: ContentType) -> Result<Self, Error> {
        let (_, pubkey_hash_base32, _) = bech32::decode(s)?;

        let pubkey_hash: [u8; Address::LEN] = Vec::<u8>::from_base32(&pubkey_hash_base32)?
            .as_slice()
            .try_into()?;

        Ok(Self {
            hash: Bytes32::from(pubkey_hash),
            encoding: s.to_string(),
            content_type,
        })
    }
}

#[allow(clippy::from_over_into)]
impl Into<Address> for &Bech32 {
    fn into(self) -> Address {
        self.to_address()
    }
}

impl From<Address> for Bech32 {
    fn from(address: Address) -> Self {
        Bech32::new_address(FUEL_BECH32_HRP, *address)
    }
}

#[allow(clippy::from_over_into)]
impl Into<ContractId> for &Bech32 {
    fn into(self) -> ContractId {
        self.to_contract_id()
    }
}

impl From<ContractId> for Bech32 {
    fn from(contract_id: ContractId) -> Self {
        Bech32::new_contract_id(FUEL_BECH32_HRP, *contract_id)
    }
}

impl Display for Bech32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encoding)
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
        let expected_bech32 = "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7";

        let bech32_addr = Bech32::new(FUEL_BECH32_HRP, pubkey_hash, ContentType::Address);

        assert_eq!(bech32_addr.to_address(), Address::new(pubkey_hash));
        assert_eq!(bech32_addr.to_address().to_string(), expected_bech32)
    }

    #[tokio::test]
    async fn test_from_str() {
        let pubkey_hash = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];

        let bech32_addr = Bech32::new_contract_id_from_string(
            "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7",
        )
        .unwrap();

        let expected_contract_id = ContractId::new(pubkey_hash);

        assert_eq!(bech32_addr.to_contract_id(), expected_contract_id);
        assert_eq!(
            bech32_addr.to_contract_id().to_string(),
            expected_contract_id.to_string()
        );
    }
}
