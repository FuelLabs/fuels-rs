use crate::{Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use std::fmt::{Debug, Display, Formatter};

// To be used when interacting with contracts which have strings in their ABI.
// The length of a string is part of its type -- i.e. str[2] is a
// different type from str[3]. The FuelVM strings only support ascii characters.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct RawSlice {
    data: Vec<u64>,
}

impl RawSlice {
    pub fn new(data: Vec<u64>) -> Result<Self, Error> {
        Ok(Self { data })
    }
}
impl Parameterize for RawSlice {
    fn param_type() -> ParamType {
        ParamType::RawSlice
    }
}

impl Tokenizable for RawSlice {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::RawSlice(contents) => Self::new(contents),
            _ => Err(Error::InvalidData(format!(
                "RawSlice::from_token expected a token of \
                the variant Token::RawSlice, got: {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::RawSlice(self.data)
    }
}

impl TryFrom<Vec<u64>> for RawSlice {
    type Error = Error;

    fn try_from(data: Vec<u64>) -> Result<Self, Self::Error> {
        Self::new(data)
    }
}

impl From<RawSlice> for Vec<u64> {
    fn from(raw_slice: RawSlice) -> Self {
        raw_slice.data
    }
}

impl Display for RawSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

impl PartialEq<Vec<u64>> for RawSlice {
    fn eq(&self, other: &Vec<u64>) -> bool {
        self.data == *other
    }
}
impl PartialEq<RawSlice> for Vec<u64> {
    fn eq(&self, other: &RawSlice) -> bool {
        *self == other.data
    }
}
