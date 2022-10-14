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
            pub fn new<T: Into<[u8; 32]>>(hrp: &str, hash: T) -> Self {
                Self {
                    hrp: hrp.to_string(),
                    hash: Bytes32::from(hash.into()),
                }
            }

            pub fn hash(&self) -> Bytes32 {
                self.hash
            }

            pub fn hrp(&self) -> &str {
                &self.hrp
            }

            fn try_from_decoded_bech32(
                hrp: String,
                pubkey_hash_base32: &[bech32::u5],
            ) -> Result<Self, Error> {
                let pubkey_hash: [u8; Bytes32::LEN] = Vec::<u8>::from_base32(pubkey_hash_base32)?
                    .as_slice()
                    .try_into()?;

                Ok(Self {
                    hrp,
                    hash: Bytes32::new(pubkey_hash),
                })
            }

            fn try_from_hex_str(s: &str) -> Result<Self, Error> {
                let hash = Bytes32::from_str(s)?;

                Ok(Self {
                    hrp: FUEL_BECH32_HRP.to_string(),
                    hash,
                })
            }
        }

        impl FromStr for $i {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let decode_result = bech32::decode(s);

                match decode_result {
                    Ok((hrp, pubkey_hash_base32, _)) => {
                        Self::try_from_decoded_bech32(hrp, &pubkey_hash_base32)
                    }
                    Err(e) if matches!(e, bech32::Error::InvalidChecksum) => Err(e.into()),
                    // only try decoding as hex string if failure wasn't due to checksum
                    Err(_) => Self::try_from_hex_str(s),
                }
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
            107, 50, 223, 89, 84, 225, 186, 222, 175, 254, 253, 44, 15, 197, 229, 148, 220, 255,
            55, 19, 170, 227, 221, 24, 183, 217, 102, 98, 75, 1, 0, 39,
        ];

        {
            // Create from Bytes32
            let bech32_addr = &Bech32Address::new(FUEL_BECH32_HRP, Bytes32::new(pubkey_hash));
            let bech32_cid = &Bech32ContractId::new(FUEL_BECH32_HRP, Bytes32::new(pubkey_hash));

            assert_eq!(*bech32_addr.hash(), pubkey_hash);
            assert_eq!(*bech32_cid.hash(), pubkey_hash);
        }

        {
            // Create from ContractId
            let bech32_addr = &Bech32Address::new(FUEL_BECH32_HRP, ContractId::new(pubkey_hash));
            let bech32_cid = &Bech32ContractId::new(FUEL_BECH32_HRP, ContractId::new(pubkey_hash));

            assert_eq!(*bech32_addr.hash(), pubkey_hash);
            assert_eq!(*bech32_cid.hash(), pubkey_hash);
        }

        {
            // Create from Address
            let bech32_addr = &Bech32Address::new(FUEL_BECH32_HRP, Address::new(pubkey_hash));
            let bech32_cid = &Bech32ContractId::new(FUEL_BECH32_HRP, Address::new(pubkey_hash));

            assert_eq!(*bech32_addr.hash(), pubkey_hash);
            assert_eq!(*bech32_cid.hash(), pubkey_hash);
        }
    }

    #[test]
    fn test_from_str() {
        let pubkey_hashes = [
            [
                107, 50, 223, 89, 84, 225, 186, 222, 175, 254, 253, 44, 15, 197, 229, 148, 220,
                255, 55, 19, 170, 227, 221, 24, 183, 217, 102, 98, 75, 1, 0, 39,
            ],
            [
                49, 83, 18, 64, 150, 242, 119, 146, 83, 184, 84, 96, 160, 212, 110, 69, 81, 34,
                101, 86, 182, 99, 62, 68, 44, 28, 40, 26, 131, 21, 221, 64,
            ],
            [
                48, 101, 49, 52, 48, 102, 48, 55, 48, 100, 49, 97, 102, 117, 51, 57, 49, 50, 48,
                54, 48, 98, 48, 100, 48, 56, 49, 53, 48, 52, 49, 52,
            ],
        ];
        let bech32m_encodings = [
            "fuel1dved7k25uxadatl7l5kql309jnw07dcn4t3a6x9hm9nxyjcpqqns50p7n2",
            "fuel1x9f3ysyk7fmey5ac23s2p4rwg4gjye2kke3nu3pvrs5p4qc4m4qqwx56k3",
            "fuel1xpjnzdpsvccrwvryx9skvafn8ycnyvpkxp3rqeps8qcn2vp5xy6qu7yyz7",
        ];
        let hex_encodings = [
            "0x6b32df5954e1badeaffefd2c0fc5e594dcff3713aae3dd18b7d966624b010027",
            "0x3153124096f2779253b85460a0d46e4551226556b6633e442c1c281a8315dd40",
            "0x3065313430663037306431616675333931323036306230643038313530343134 ",
        ];

        {
            for i in 0..3 {
                let bech32_contract_id = &Bech32ContractId::from_str(bech32m_encodings[i]).unwrap();
                assert_eq!(*bech32_contract_id.hash(), pubkey_hashes[i]);
            }

            for i in 0..3 {
                let bech32_contract_id = &Bech32ContractId::from_str(hex_encodings[i]).unwrap();
                assert_eq!(*bech32_contract_id.hash(), pubkey_hashes[i]);
            }
        }

        {
            for i in 0..3 {
                let bech32_contract_id = &Bech32Address::from_str(bech32m_encodings[i]).unwrap();
                assert_eq!(*bech32_contract_id.hash(), pubkey_hashes[i]);
            }

            for i in 0..3 {
                let bech32_contract_id = &Bech32Address::from_str(hex_encodings[i]).unwrap();
                assert_eq!(*bech32_contract_id.hash(), pubkey_hashes[i]);
            }
        }
    }

    #[test]
    fn test_from_invalid_bech32_string() {
        {
            let expected: Error = bech32::Error::InvalidChecksum.into();
            let invalid_bech32_with_hex_chars =
                "fadeaffe1dfed7c25adadadd715e3f309ecf07dca4e3a6d9de9cdadcddac50d7f";

            {
                let result = &Bech32ContractId::from_str(invalid_bech32_with_hex_chars)
                    .expect_err("should error");
                assert_eq!(result.to_string(), expected.to_string());
            }

            {
                let result = &Bech32Address::from_str(invalid_bech32_with_hex_chars)
                    .expect_err("should error");
                assert_eq!(result.to_string(), expected.to_string());
            }
        }
    }
}
