use crate::errors::Error;

// A simple wrapper around [u8; 32] representing the `b256` type. Exists
// mainly so that we may differentiate `Parameterize` and `Tokenizable`
// implementations from what otherwise is just an array of 32 u8's.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bits256(pub [u8; 32]);

impl Bits256 {
    /// Create a new `Bits256` from a string representation of a hex.
    /// Accepts both `0x` prefixed and non-prefixed hex strings.
    pub fn from_hex_str(hex: &str) -> Result<Self, Error> {
        let hex = if let Some(stripped_hex) = hex.strip_prefix("0x") {
            stripped_hex
        } else {
            hex
        };

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(hex, &mut bytes as &mut [u8]).map_err(|e| {
            Error::InvalidData(format!("Could not convert hex str '{hex}' to Bits256! {e}"))
        })?;
        Ok(Bits256(bytes))
    }
}

// A simple wrapper around [Bits256; 2] representing the `B512` type.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Ok(B512 {
            bytes: [
                Bits256(slice[0..32].try_into()?),
                Bits256(slice[32..].try_into()?),
            ],
        })
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
// ANCHOR: evm_address
pub struct EvmAddress {
    // An evm address is only 20 bytes, the first 12 bytes should be set to 0
    pub value: Bits256,
}
// ANCHOR_END: evm_address

impl EvmAddress {
    // sets the leftmost 12 bytes to zero
    fn clear_12_bytes(bytes: [u8; 32]) -> [u8; 32] {
        let mut bytes = bytes;
        bytes[..12].copy_from_slice(&[0u8; 12]);

        bytes
    }
}

impl From<Bits256> for EvmAddress {
    fn from(b256: Bits256) -> Self {
        let value = Bits256(Self::clear_12_bytes(b256.0));

        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_str_b256() -> Result<(), Error> {
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
}
