use crate::{Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

pub struct Byte(pub u8);

impl Parameterize for Byte {
    fn param_type() -> ParamType {
        ParamType::Byte
    }
}

impl Tokenizable for Byte {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::Byte(value) => Ok(Byte(value)),
            _ => Err(Error::InvalidData(format!(
                "Byte::from_token failed! Can only handle Token::Byte, got {token:?}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Byte(self.0)
    }
}
