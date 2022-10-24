use crate::{Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

// A simple wrapper around [u8;32] representing the `b256` type. Exists
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

impl Parameterize for Bits256 {
    fn param_type() -> ParamType {
        ParamType::B256
    }
}

impl Tokenizable for Bits256 {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::B256(data) => Ok(Bits256(data)),
            _ => Err(Error::InvalidData(format!(
                "Bits256 cannot be constructed from token {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::B256(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizable;
    use fuels_types::param_types::ParamType;

    #[test]
    fn test_param_type() {
        assert_eq!(Bits256::param_type(), ParamType::B256);
    }

    #[test]
    fn test_from_token() -> Result<(), Error> {
        let data = [0u8; 32];
        let token = Token::B256(data);

        let bits256 = Bits256::from_token(token)?;

        assert_eq!(bits256.0, data);

        Ok(())
    }

    #[test]
    fn test_into_token() {
        let data = [0u8; 32];
        let bits256 = Bits256(data);

        let token = bits256.into_token();

        assert_eq!(token, Token::B256(data));
    }

    #[test]
    fn from_hex_str() -> Result<(), Error> {
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
