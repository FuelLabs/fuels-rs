use std::fmt;

use crate::types::{
    core::U256,
    errors::{Error, Result, error},
    param_types::EnumVariants,
};

pub type EnumSelector = (u64, Token, EnumVariants);

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct StaticStringToken {
    pub(crate) data: String,
    expected_len: Option<usize>,
}

impl StaticStringToken {
    pub fn new(data: String, expected_len: Option<usize>) -> Self {
        StaticStringToken { data, expected_len }
    }

    fn validate(&self) -> Result<()> {
        if !self.data.is_ascii() {
            return Err(error!(Codec, "string data can only have ascii values"));
        }

        if let Some(expected_len) = self.expected_len {
            if self.data.len() != expected_len {
                return Err(error!(
                    Codec,
                    "string data has len {}, but the expected len is {}",
                    self.data.len(),
                    expected_len
                ));
            }
        }

        Ok(())
    }

    pub fn get_encodable_str(&self) -> Result<&str> {
        self.validate()?;
        Ok(self.data.as_str())
    }
}

impl TryFrom<StaticStringToken> for String {
    type Error = Error;
    fn try_from(string_token: StaticStringToken) -> Result<String> {
        string_token.validate()?;
        Ok(string_token.data)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Token {
    // Used for unit type variants in Enum. An "empty" enum is not represented as Enum<empty box>,
    // because this way we can have both unit and non-unit type variants.
    Unit,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    B256([u8; 32]),
    Bytes(Vec<u8>),
    String(String),
    RawSlice(Vec<u8>),
    StringArray(StaticStringToken),
    StringSlice(StaticStringToken),
    Tuple(Vec<Token>),
    Array(Vec<Token>),
    Vector(Vec<Token>),
    Struct(Vec<Token>),
    Enum(Box<EnumSelector>),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for Token {
    fn default() -> Self {
        Token::U8(0)
    }
}

impl Token {
    /// Returns true if this [Token] is an exact-size ABI type.
    pub fn is_exact_size_abi(&self) -> bool {
        match self {
            Token::Unit
            | Token::Bool(_)
            | Token::U8(_) | Token::U16(_) | Token::U32(_) | Token::U64(_) | Token::U128(_) | Token::U256(_)
            | Token::B256(_) => true,

            // Dynamic or heap-allocated
            Token::Bytes(_) | Token::String(_) | Token::RawSlice(_)
            | Token::StringArray(_) | Token::StringSlice(_)
            | Token::Vector(_) => false,

            // Nested container: all elements must be exact-size
            Token::Tuple(elems) | Token::Array(elems) | Token::Struct(elems) => {
                elems.iter().all(|t| t.is_exact_size_abi())
            }

            // Enum: second element of selector is the payload Token
            Token::Enum(selector) => selector.1.is_exact_size_abi(),
        }
    }
}

mod tests {


    #[test]
    fn primitives() {
        assert!(Token::U32(0).is_exact_size_abi());
        assert!(Token::B256([0u8; 32]).is_exact_size_abi());
        assert!(!Token::String("a".into()).is_exact_size_abi());
    }

    #[test]
    fn nested() {
        let good = Token::Tuple(vec![Token::U16(1), Token::Bool(true)]);
        assert!(good.is_exact_size_abi());

        let bad = Token::Struct(vec![Token::U8(2), Token::Bytes(vec![1])]);
        assert!(!bad.is_exact_size_abi());
    }

    // #[test]
    // fn enum_token() {
    //     let ok = Token::Enum(Box::new((0, Token::U64(5), EnumVariants::default())));
    //     assert!(ok.is_exact_size_abi());

    //     let err = Token::Enum(Box::new((1, Token::String("e".into()), EnumVariants::default())));
    //     assert!(!err.is_exact_size_abi());
    // }
}
