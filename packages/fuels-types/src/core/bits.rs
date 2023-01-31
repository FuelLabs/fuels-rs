use fuels_macros::{Parameterize, Tokenizable};

use crate::errors::{error, Error, Result};

// A simple wrapper around [u8; 32] representing the `b256` type. Exists
// mainly so that we may differentiate `Parameterize` and `Tokenizable`
// implementations from what otherwise is just an array of 32 u8's.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bits256(pub [u8; 32]);

impl Bits256 {
    /// Create a new `Bits256` from a string representation of a hex.
    /// Accepts both `0x` prefixed and non-prefixed hex strings.
    pub fn from_hex_str(hex: &str) -> Result<Self> {
        let hex = if let Some(stripped_hex) = hex.strip_prefix("0x") {
            stripped_hex
        } else {
            hex
        };

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(hex, &mut bytes as &mut [u8]).map_err(|e| {
            error!(
                InvalidData,
                "Could not convert hex str '{hex}' to Bits256! {e}"
            )
        })?;
        Ok(Bits256(bytes))
    }
}

// A simple wrapper around [Bits256; 2] representing the `B512` type.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Parameterize, Tokenizable)]
#[FuelsTypesPath("crate")]
// ANCHOR: b512
pub struct B512 {
    pub bytes: [Bits256; 2],
}
// ANCHOR_END: b512

impl From<(Bits256, Bits256)> for B512 {
    fn from(bits_tuple: (Bits256, Bits256)) -> Self {
        B512 {
            bytes: [bits_tuple.0, bits_tuple.1],
        }
    }
}

impl TryFrom<&[u8]> for B512 {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self> {
        Ok(B512 {
            bytes: [
                Bits256(slice[0..32].try_into()?),
                Bits256(slice[32..].try_into()?),
            ],
        })
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Parameterize, Tokenizable)]
#[FuelsTypesPath("crate")]
// ANCHOR: evm_address
pub struct EvmAddress {
    // An evm address is only 20 bytes, the first 12 bytes should be set to 0
    value: Bits256,
}
// ANCHOR_END: evm_address
impl EvmAddress {
    fn new(b256: Bits256) -> Self {
        Self {
            value: Bits256(Self::clear_12_bytes(b256.0)),
        }
    }

    pub fn value(&self) -> Bits256 {
        self.value
    }

    // sets the leftmost 12 bytes to zero
    fn clear_12_bytes(bytes: [u8; 32]) -> [u8; 32] {
        let mut bytes = bytes;
        bytes[..12].copy_from_slice(&[0u8; 12]);

        bytes
    }
}

impl From<Bits256> for EvmAddress {
    fn from(b256: Bits256) -> Self {
        EvmAddress::new(b256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        param_types::ParamType,
        traits::{Parameterize, Tokenizable},
        Token,
    };

    #[test]
    fn from_hex_str_b256() -> Result<()> {
        // ANCHOR: from_hex_str
        let hex_str = "0101010101010101010101010101010101010101010101010101010101010101";

        let bits256 = Bits256::from_hex_str(hex_str)?;

        assert_eq!(bits256.0, [1u8; 32]);

        // With the `0x0` prefix
        let hex_str = "0x0101010101010101010101010101010101010101010101010101010101010101";

        let bits256 = Bits256::from_hex_str(hex_str)?;

        assert_eq!(bits256.0, [1u8; 32]);
        // ANCHOR_END: from_hex_str

        Ok(())
    }

    #[test]
    fn test_param_type_evm_addr() {
        assert_eq!(
            EvmAddress::param_type(),
            ParamType::Struct {
                name: "EvmAddress".to_string(),
                fields: vec![("value".to_string(), ParamType::B256)],
                generics: vec![]
            }
        );
    }

    #[test]
    fn evm_address_clears_first_12_bytes() -> Result<()> {
        let data = [1u8; 32];
        let address = EvmAddress::new(Bits256(data));

        let expected_data = Bits256([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ]);

        assert_eq!(address.value(), expected_data);

        Ok(())
    }

    #[test]
    fn test_into_token_evm_addr() {
        let bits = [1u8; 32];
        let evm_address = EvmAddress::from(Bits256(bits));

        let token = evm_address.into_token();

        let expected_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];

        assert_eq!(token, Token::Struct(vec![Token::B256(expected_data)]));
    }
}
