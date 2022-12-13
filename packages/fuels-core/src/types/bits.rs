use crate::{Parameterize, Token, Tokenizable};
use fuels_types::{errors::Error, param_types::ParamType};

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

impl Parameterize for B512 {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "B512".to_string(),
            fields: vec![("bytes".to_string(), <[Bits256; 2usize]>::param_type())],
            generics: vec![],
        }
    }
}

impl Tokenizable for B512 {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            if let [Token::Array(data)] = tokens.as_slice() {
                Ok(B512 {
                    bytes: <[Bits256; 2usize]>::from_token(Token::Array(data.to_vec()))?,
                })
            } else {
                Err(Error::InstantiationError(format!(
                    "B512 expected one `Token::Array`, got {tokens:?}",
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "B512 expected `Token::Struct`, got {token:?}",
            )))
        }
    }

    fn into_token(self) -> Token {
        Token::Struct(vec![self.bytes.into_token()])
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

impl Parameterize for EvmAddress {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "EvmAddress".to_string(),
            fields: vec![("value".to_string(), ParamType::B256)],
            generics: vec![],
        }
    }
}

impl Tokenizable for EvmAddress {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            if let [Token::B256(data)] = tokens.as_slice() {
                Ok(EvmAddress::from(Bits256(*data)))
            } else {
                Err(Error::InstantiationError(format!(
                    "EvmAddress expected one `Token::B256`, got {tokens:?}",
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "EvmAddress expected `Token::Struct` got {token:?}",
            )))
        }
    }

    fn into_token(self) -> Token {
        Token::Struct(vec![Bits256(self.value.0).into_token()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizable;
    use fuels_types::param_types::ParamType;

    #[test]
    fn test_param_type_b256() {
        assert_eq!(Bits256::param_type(), ParamType::B256);
    }

    #[test]
    fn test_from_token_b256() -> Result<(), Error> {
        let data = [1u8; 32];
        let token = Token::B256(data);

        let bits256 = Bits256::from_token(token)?;

        assert_eq!(bits256.0, data);

        Ok(())
    }

    #[test]
    fn test_into_token_b256() {
        let data = [1u8; 32];
        let bits256 = Bits256(data);

        let token = bits256.into_token();

        assert_eq!(token, Token::B256(data));
    }

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
    fn test_from_token_evm_addr() -> Result<(), Error> {
        let data = [1u8; 32];
        let token = Token::Struct(vec![Token::B256(data)]);

        let evm_address = EvmAddress::from_token(token)?;

        let expected_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];

        assert_eq!(evm_address.value.0, expected_data);

        Ok(())
    }

    #[test]
    fn test_into_token_evm_addr() {
        let data = [1u8; 32];
        let evm_address = EvmAddress::from(Bits256(data));

        let token = evm_address.into_token();

        let expected_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];

        assert_eq!(token, Token::Struct(vec![Token::B256(expected_data)]));
    }
}
