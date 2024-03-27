use crate::types::errors::Result;

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    /// Create a new `Bytes` from a string representation of a hex.
    /// Accepts both `0x` prefixed and non-prefixed hex strings.
    pub fn from_hex_str(hex: &str) -> Result<Self> {
        let hex = if let Some(stripped_hex) = hex.strip_prefix("0x") {
            stripped_hex
        } else {
            hex
        };
        let bytes = hex::decode(hex)?;

        Ok(Bytes(bytes))
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(bytes: Bytes) -> Vec<u8> {
        bytes.0
    }
}

impl PartialEq<Vec<u8>> for Bytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Bytes> for Vec<u8> {
    fn eq(&self, other: &Bytes) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hex_str_b256() -> Result<()> {
        // ANCHOR: bytes_from_hex_str
        let hex_str = "0101010101010101010101010101010101010101010101010101010101010101";

        let bytes = Bytes::from_hex_str(hex_str)?;

        assert_eq!(bytes.0, vec![1u8; 32]);

        // With the `0x0` prefix
        // ANCHOR: hex_string_to_bytes32
        let hex_str = "0x0101010101010101010101010101010101010101010101010101010101010101";

        let bytes = Bytes::from_hex_str(hex_str)?;
        // ANCHOR_END: hex_string_to_bytes32

        assert_eq!(bytes.0, vec![1u8; 32]);
        // ANCHOR_END: bytes_from_hex_str

        Ok(())
    }
}
