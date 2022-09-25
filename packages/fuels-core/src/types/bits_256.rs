use crate::{Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

// A simple wrapper around [u8;32] representing the `b256` type. Exists
// mainly so that we may differentiate `Parameterize` and `Tokenizable`
// implementations from what otherwise is just an array of 32 u8's.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bits256(pub [u8; 32]);

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
