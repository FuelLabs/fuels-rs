use std::fmt::{Debug, Display, Formatter};

use crate::types::errors::{error, Error, Result};
use serde::{Deserialize, Serialize};

// To be used when interacting with contracts which have string slices in their ABI.
// The FuelVM strings only support ascii characters.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct AsciiString {
    data: String,
}

impl AsciiString {
    pub fn new(data: String) -> Result<Self> {
        if !data.is_ascii() {
            return Err(error!(InvalidData,
                "AsciiString must be constructed from a string containing only ascii encodable characters. Got: {data}"
            ));
        }
        Ok(Self { data })
    }

    pub fn to_trimmed_str(&self) -> &str {
        self.data.trim()
    }
    pub fn to_left_trimmed_str(&self) -> &str {
        self.data.trim_start()
    }
    pub fn to_right_trimmed_str(&self) -> &str {
        self.data.trim_end()
    }
}

impl TryFrom<&str> for AsciiString {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        Self::new(value.to_owned())
    }
}

impl TryFrom<String> for AsciiString {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<AsciiString> for String {
    fn from(ascii_str: AsciiString) -> Self {
        ascii_str.data
    }
}

impl Display for AsciiString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl PartialEq<&str> for AsciiString {
    fn eq(&self, other: &&str) -> bool {
        self.data == *other
    }
}
impl PartialEq<AsciiString> for &str {
    fn eq(&self, other: &AsciiString) -> bool {
        *self == other.data
    }
}

// To be used when interacting with contracts which have strings in their ABI.
// The length of a string is part of its type -- i.e. str[2] is a
// different type from str[3]. The FuelVM strings only support ascii characters.
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct SizedAsciiString<const LEN: usize> {
    data: String,
}

impl<const LEN: usize> SizedAsciiString<LEN> {
    pub fn new(data: String) -> Result<Self> {
        if !data.is_ascii() {
            return Err(error!(InvalidData,
                "SizedAsciiString must be constructed from a string containing only ascii encodable characters. Got: {data}"
            ));
        }
        if data.len() != LEN {
            return Err(error!(InvalidData,
                "SizedAsciiString<{LEN}> can only be constructed from a String of length {LEN}. Got: {data}"
            ));
        }
        Ok(Self { data })
    }

    pub fn to_trimmed_str(&self) -> &str {
        self.data.trim()
    }
    pub fn to_left_trimmed_str(&self) -> &str {
        self.data.trim_start()
    }
    pub fn to_right_trimmed_str(&self) -> &str {
        self.data.trim_end()
    }

    /// Pad `data` string with whitespace characters on the right to fit into the `SizedAsciiString`
    pub fn new_with_right_whitespace_padding(data: String) -> Result<Self> {
        if data.len() > LEN {
            return Err(error!(
                InvalidData,
                "SizedAsciiString<{LEN}> cannot be constructed from a string of size {}",
                data.len()
            ));
        }

        Ok(Self {
            data: format!("{:LEN$}", data),
        })
    }
}

impl<const LEN: usize> TryFrom<&str> for SizedAsciiString<LEN> {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        Self::new(value.to_owned())
    }
}

impl<const LEN: usize> TryFrom<String> for SizedAsciiString<LEN> {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
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

impl<const LEN: usize> Serialize for SizedAsciiString<LEN> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> core::result::Result<S::Ok, S::Error> {
        self.data.serialize(serializer)
    }
}

impl<'de, const LEN: usize> Deserialize<'de> for SizedAsciiString<LEN> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> core::result::Result<Self, D::Error> {
        let data = String::deserialize(deserializer)?;
        Self::new(data).map_err(serde::de::Error::custom)
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

    #[test]
    fn trim() -> Result<()> {
        // Using single whitespaces
        let untrimmed = SizedAsciiString::<9>::new(" est abc ".to_string())?;
        assert_eq!("est abc ", untrimmed.to_left_trimmed_str());
        assert_eq!(" est abc", untrimmed.to_right_trimmed_str());
        assert_eq!("est abc", untrimmed.to_trimmed_str());

        let padded = // adds 6 whitespaces
            SizedAsciiString::<12>::new_with_right_whitespace_padding("victor".to_string())?;
        assert_eq!("victor      ", padded);

        Ok(())
    }

    #[test]
    fn test_can_serialize_sized_ascii() {
        let sized_str = SizedAsciiString::<3>::new("abc".to_string()).unwrap();

        let serialized = serde_json::to_string(&sized_str).unwrap();
        assert_eq!(serialized, "\"abc\"");
    }

    #[test]
    fn test_can_deserialize_sized_ascii() {
        let serialized = "\"abc\"";

        let deserialized: SizedAsciiString<3> = serde_json::from_str(serialized).unwrap();
        assert_eq!(
            deserialized,
            SizedAsciiString::<3>::new("abc".to_string()).unwrap()
        );
    }
}
