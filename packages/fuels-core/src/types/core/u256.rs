#![allow(clippy::assign_op_pattern)]

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use uint::construct_uint;

use crate::{
    traits::{Parameterize, Tokenizable},
    types::{
        errors::{error, Result as FuelsResult},
        param_types::ParamType,
        Token,
    },
};

construct_uint! {
    pub struct U256(4);
}

impl Parameterize for U256 {
    fn param_type() -> ParamType {
        ParamType::U256
    }
}

impl Tokenizable for U256 {
    fn from_token(token: Token) -> FuelsResult<Self>
    where
        Self: Sized,
    {
        match token {
            Token::U256(data) => Ok(data),
            _ => Err(error!(
                Other,
                "`U256` cannot be constructed from token `{token}`"
            )),
        }
    }

    fn into_token(self) -> Token {
        Token::U256(self)
    }
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for U256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        U256::from_dec_str(Deserialize::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::U256;

    #[test]
    fn u256_serialize_deserialize() {
        let num = U256::from(123);
        let serialized: String = serde_json::to_string(&num).unwrap();
        assert_eq!(serialized, "\"123\"");

        let deserialized_num: U256 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized_num, num);
    }
}
