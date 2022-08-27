use crate::errors::Error;
use bech32::Variant::Bech32m;
use bech32::{FromBase32, ToBase32};
use core::str;
use fuel_tx::{Address, Bytes32, ContractId};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

// Fuel Network human-readable part for bech32 encoding
pub const FUEL_BECH32_HRP: &str = "fuel";

/// Generate type represented in the Bech32 format,
/// consisting of a human-readable part (hrp) and a hash (e.g. pubkey-, contract hash)
macro_rules! bech32type {
    ($i:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $i {
            pub hrp: String,
            pub hash: Bytes32,
        }

        impl $i {
            pub fn new<T: Into<Bytes32>>(hrp: &str, hash: T) -> Self {
                Self {
                    hrp: hrp.to_string(),
                    hash: hash.into(),
                }
            }

            pub fn hash(&self) -> Bytes32 {
                self.hash
            }

            pub fn hrp(&self) -> &str {
                &self.hrp
            }
        }

        impl FromStr for $i {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let (hrp, pubkey_hash_base32, _) = bech32::decode(s)?;

                let pubkey_hash: [u8; Address::LEN] = Vec::<u8>::from_base32(&pubkey_hash_base32)?
                    .as_slice()
                    .try_into()?;

                Ok(Self {
                    hrp,
                    hash: Bytes32::new(pubkey_hash),
                })
            }
        }

        impl Display for $i {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let data_base32 = self.hash.to_base32();
                let encoding = bech32::encode(&self.hrp, &data_base32, Bech32m).unwrap();

                write!(f, "{}", encoding)
            }
        }
    };
}

bech32type!(Bech32Address);
bech32type!(Bech32ContractId);

// Bech32Address - Address conversion
impl From<&Bech32Address> for Address {
    fn from(data: &Bech32Address) -> Address {
        Address::new(*data.hash)
    }
}
impl From<Bech32Address> for Address {
    fn from(data: Bech32Address) -> Address {
        Address::new(*data.hash)
    }
}
impl From<Address> for Bech32Address {
    fn from(address: Address) -> Self {
        Self {
            hrp: FUEL_BECH32_HRP.to_string(),
            hash: Bytes32::new(*address),
        }
    }
}

// Bech32ContractId - ContractId conversion
impl From<&Bech32ContractId> for ContractId {
    fn from(data: &Bech32ContractId) -> ContractId {
        ContractId::new(*data.hash)
    }
}
impl From<Bech32ContractId> for ContractId {
    fn from(data: Bech32ContractId) -> ContractId {
        ContractId::new(*data.hash)
    }
}
impl From<ContractId> for Bech32ContractId {
    fn from(contract_id: ContractId) -> Self {
        Self {
            hrp: FUEL_BECH32_HRP.to_string(),
            hash: Bytes32::new(*contract_id),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let pubkey_hash = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];
        let expected_address = Address::new(pubkey_hash);

        let bech32_addr = &Bech32Address::new(FUEL_BECH32_HRP, Bytes32::new(pubkey_hash));
        let plain_addr: Address = bech32_addr.into();

        assert_eq!(plain_addr, expected_address);
    }

    #[test]
    fn test_from_str() {
        let pubkey_hash = [
            48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48, 54,
            48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
        ];

        let bech32_contract_id = &Bech32ContractId::from_str(
            "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7",
        )
        .unwrap();
        let plain_contract_id: ContractId = bech32_contract_id.into();

        let expected_contract_id = ContractId::new(pubkey_hash);

        assert_eq!(plain_contract_id, expected_contract_id);
    }
}
