use crate::{Parameterize, StringToken, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct SizedAsciiString<const LEN: usize> {
    data: String,
}

impl<const LEN: usize> SizedAsciiString<LEN> {
    pub fn new(data: String) -> Result<Self, Error> {
        if data.len() != LEN {
            return Err(Error::InvalidData(format!(
                "SizedAsciiString<{LEN}> can only be constructed from a String of length {LEN}"
            )));
        }
        if !data.is_ascii() {
            return Err(Error::InvalidData(format!(
                "SizedAsciiString<{LEN}> must be constructed from a string containing only ascii encodable characters. Got: {data}"
            )));
        }
        Ok(Self { data })
    }
}
impl<const LEN: usize> Parameterize for SizedAsciiString<LEN> {
    fn param_type() -> ParamType {
        ParamType::String(LEN)
    }
}

impl<const LEN: usize> Tokenizable for SizedAsciiString<LEN> {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::String(contents) => {
                if contents.expected_len != LEN {
                    return Err(Error::InvalidData(format!("SizedAsciiString<{LEN}>::from_token got a Token::String whose expected length({}) is != {LEN}", contents.expected_len)))
                }
                Self::new(contents.data)
            },
            _ => {
                Err(Error::InvalidData(format!("SizedAsciiString<{LEN}>::from_token expected a token of the variant Token::String, got: {token}")))
            }
        }
    }

    fn into_token(self) -> Token {
        Token::String(StringToken::new(self.data, LEN))
    }
}

impl<const LEN: usize> TryFrom<&str> for SizedAsciiString<LEN> {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_owned())
    }
}

impl<const LEN: usize> PartialEq<&str> for SizedAsciiString<LEN> {
    fn eq(&self, other: &&str) -> bool {
        self.data == *other
    }
}
impl<const LEN: usize> PartialEq<SizedAsciiString<LEN>> for &str {
    fn eq(&self, other: &SizedAsciiString<LEN>) -> bool {
        *self == other.data
    }
}
