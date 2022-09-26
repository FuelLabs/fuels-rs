use crate::{Parameterize, StringToken, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;
use std::fmt::{Debug, Display, Formatter};

// To be used when interacting with contracts which have strings in their ABI.
// The length of a string is part of its type -- i.e. str[2] is a
// different type from str[3]. The FuelVM strings only support ascii characters.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct SizedAsciiString<const LEN: usize> {
    data: String,
}

impl<const LEN: usize> SizedAsciiString<LEN> {
    pub fn new(data: String) -> Result<Self, Error> {
        if !data.is_ascii() {
            return Err(Error::InvalidData(format!(
                "SizedAsciiString must be constructed from a string containing only ascii encodable characters. Got: {data}"
            )));
        }
        if data.len() != LEN {
            return Err(Error::InvalidData(format!(
                "SizedAsciiString<{LEN}> can only be constructed from a String of length {LEN}. Got: {data}"
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

impl<const LEN: usize> TryFrom<String> for SizedAsciiString<LEN> {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const LEN: usize> From<SizedAsciiString<LEN>> for String {
    fn from(sized_ascii_str: SizedAsciiString<LEN>) -> Self {
        sized_ascii_str.data
    }
}

impl<const LEN: usize> Display for SizedAsciiString<LEN> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ascii_of_correct_length() {
        // ANCHOR: string_simple_example
        let ascii_data = "abc".to_string();

        SizedAsciiString::<3>::new(ascii_data)
            .expect("Should have succeeded since we gave ascii data of correct length!");
        // ANCHOR_END: string_simple_example
    }

    #[test]
    fn refuses_non_ascii() {
        let ascii_data = "ab©".to_string();

        let err = SizedAsciiString::<3>::new(ascii_data)
            .expect_err("Should not have succeeded since we gave non ascii data");

        let expected_reason = "SizedAsciiString must be constructed from a string containing only ascii encodable characters. Got: ";
        assert!(matches!(err, Error::InvalidData(reason) if reason.starts_with(expected_reason)));
    }

    #[test]
    fn refuses_invalid_len() {
        let ascii_data = "abcd".to_string();

        let err = SizedAsciiString::<3>::new(ascii_data)
            .expect_err("Should not have succeeded since we gave data of wrong length");

        let expected_reason =
            "SizedAsciiString<3> can only be constructed from a String of length 3. Got: abcd";
        assert!(matches!(err, Error::InvalidData(reason) if reason.starts_with(expected_reason)));
    }

    #[test]
    fn is_parameterized_correctly() {
        let param_type = SizedAsciiString::<3>::param_type();

        assert!(matches!(param_type, ParamType::String(3)));
    }

    #[test]
    fn is_tokenized_correctly() -> anyhow::Result<()> {
        let sut = SizedAsciiString::<3>::new("abc".to_string())?;

        let token = sut.into_token();

        match token {
            Token::String(string_token) => {
                assert_eq!(string_token.data, "abc");
                assert_eq!(string_token.expected_len, 3);
            }
            _ => {
                panic!("Not tokenized correctly! Should have gotten a Token::String")
            }
        }

        Ok(())
    }

    #[test]
    fn is_detokenized_correctly() -> anyhow::Result<()> {
        let token = Token::String(StringToken {
            data: "abc".to_string(),
            expected_len: 3,
        });

        let sized_ascii_string =
            SizedAsciiString::<3>::from_token(token).expect("Should have succeeded");

        assert_eq!(sized_ascii_string.data, "abc");

        Ok(())
    }

    // ANCHOR: conversion
    #[test]
    fn can_be_constructed_from_str_ref() {
        let _: SizedAsciiString<3> = "abc".try_into().expect("Should have succeeded");
    }

    #[test]
    fn can_be_constructed_from_string() {
        let _: SizedAsciiString<3> = "abc".to_string().try_into().expect("Should have succeeded");
    }

    #[test]
    fn can_be_converted_into_string() {
        let sized_str = SizedAsciiString::<3>::new("abc".to_string()).unwrap();

        let str: String = sized_str.into();

        assert_eq!(str, "abc");
    }
    // ANCHOR_END: conversion

    #[test]
    fn can_be_printed() {
        let sized_str = SizedAsciiString::<3>::new("abc".to_string()).unwrap();

        assert_eq!(sized_str.to_string(), "abc");
    }

    #[test]
    fn can_be_compared_w_str_ref() {
        let sized_str = SizedAsciiString::<3>::new("abc".to_string()).unwrap();

        assert_eq!(sized_str, "abc");
        // and vice-versa
        assert_eq!("abc", sized_str);
    }
}
