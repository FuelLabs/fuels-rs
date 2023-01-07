use crate::{Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

#[derive(Debug, PartialEq, Clone, Eq)]
// `RawSlice` is a mapping of the contract type "untyped raw slice" -- currently the only way of
// returning dynamically sized data from a script.
pub struct RawSlice(pub Vec<u64>);

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
            Token::RawSlice(contents) => Ok(Self(contents)),
            _ => Err(Error::InvalidData(format!(
                "RawSlice::from_token expected a token of the variant Token::RawSlice, got: {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::RawSlice(Vec::from(self))
    }
}

impl From<RawSlice> for Vec<u64> {
    fn from(raw_slice: RawSlice) -> Vec<u64> {
        raw_slice.0
    }
}

impl PartialEq<Vec<u64>> for RawSlice {
    fn eq(&self, other: &Vec<u64>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<RawSlice> for Vec<u64> {
    fn eq(&self, other: &RawSlice) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizable;
    use fuels_types::param_types::ParamType;

    #[test]
    fn test_param_type_raw_slice() {
        assert_eq!(RawSlice::param_type(), ParamType::RawSlice);
    }

    #[test]
    fn test_from_token_raw_slice() -> Result<(), Error> {
        let data = vec![42; 11];
        let token = Token::RawSlice(data.clone());

        let slice = RawSlice::from_token(token)?;

        assert_eq!(slice.0, data);

        Ok(())
    }

    #[test]
    fn test_into_token_raw_slice() {
        let data = vec![13; 32];
        let raw_slice_token = Token::RawSlice(data.clone());

        let token = raw_slice_token.into_token();

        assert_eq!(token, Token::RawSlice(data));
    }
}
