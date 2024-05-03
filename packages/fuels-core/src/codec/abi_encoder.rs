mod bounded_encoder;

use std::default::Default;

use crate::{
    codec::abi_encoder::bounded_encoder::BoundedEncoder,
    types::{errors::Result, Token},
};

#[derive(Debug, Clone, Copy)]
pub struct EncoderConfig {
    /// Entering a struct, array, tuple, enum or vector increases the depth. Encoding will fail if
    /// the current depth becomes greater than `max_depth` configured here.
    pub max_depth: usize,
    /// Every encoded argument will increase the token count. Encoding will fail if the current
    /// token count becomes greater than `max_tokens` configured here.
    pub max_tokens: usize,
}

// ANCHOR: default_encoder_config
impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            max_depth: 45,
            max_tokens: 10_000,
        }
    }
}
// ANCHOR_END: default_encoder_config

#[derive(Default, Clone, Debug)]
pub struct ABIEncoder {
    pub config: EncoderConfig,
}

impl ABIEncoder {
    pub fn new(config: EncoderConfig) -> Self {
        Self { config }
    }

    /// Encodes `Token`s following the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md)
    pub fn encode(&self, tokens: &[Token]) -> Result<Vec<u8>> {
        BoundedEncoder::new(self.config).encode(tokens)
    }
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::{
        to_named,
        types::{
            errors::Error,
            param_types::{EnumVariants, ParamType},
            StaticStringToken, U256,
        },
    };

    #[test]
    fn encode_multiple_uint() -> Result<()> {
        let tokens = [
            Token::U8(u8::MAX),
            Token::U16(u16::MAX),
            Token::U32(u32::MAX),
            Token::U64(u64::MAX),
            Token::U128(u128::MAX),
            Token::U256(U256::MAX),
        ];

        let result = ABIEncoder::default().encode(&tokens)?;

        let expected = [
            255, // u8
            255, 255, // u16
            255, 255, 255, 255, // u32
            255, 255, 255, 255, 255, 255, 255, 255, // u64
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, // u128
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, // u256
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_bool() -> Result<()> {
        let token = Token::Bool(true);

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [1];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_b256() -> Result<()> {
        let data = [
            213, 87, 156, 70, 223, 204, 127, 24, 32, 112, 19, 230, 91, 68, 228, 203, 78, 44, 34,
            152, 244, 172, 69, 123, 168, 248, 39, 67, 243, 30, 147, 11,
        ];
        let token = Token::B256(data);

        let result = ABIEncoder::default().encode(&[token])?;

        assert_eq!(result, data);

        Ok(())
    }

    #[test]
    fn encode_bytes() -> Result<()> {
        let token = Token::Bytes([255, 0, 1, 2, 3, 4, 5].to_vec());

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 7, // len
            255, 0, 1, 2, 3, 4, 5, // data
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_string() -> Result<()> {
        let token = Token::String("This is a full sentence".to_string());

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 23, // len
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_raw_slice() -> Result<()> {
        let token = Token::RawSlice([255, 0, 1, 2, 3, 4, 5].to_vec());

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 7, // len
            255, 0, 1, 2, 3, 4, 5, // data
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_string_array() -> Result<()> {
        let token = Token::StringArray(StaticStringToken::new(
            "This is a full sentence".into(),
            Some(23),
        ));

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_string_slice() -> Result<()> {
        let token = Token::StringSlice(StaticStringToken::new(
            "This is a full sentence".into(),
            None,
        ));

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 23, // len
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_tuple() -> Result<()> {
        let token = Token::Tuple(vec![Token::U32(255), Token::Bool(true)]);

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 255, //u32
            1,   //bool
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_array() -> Result<()> {
        let token = Token::Tuple(vec![Token::U32(255), Token::U32(128)]);

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 0, 0, 255, //u32
            0, 0, 0, 128, //u32
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_enum_with_deeply_nested_types() -> Result<()> {
        /*
        enum DeeperEnum {
            v1: bool,
            v2: str[10]
        }
         */
        let types = to_named(&[ParamType::Bool, ParamType::StringArray(10)]);
        let deeper_enum_variants = EnumVariants::new(types)?;
        let deeper_enum_token =
            Token::StringArray(StaticStringToken::new("0123456789".into(), Some(10)));

        /*
        struct StructA {
            some_enum: DeeperEnum
            some_number: u32
        }
         */

        let fields = to_named(&[
            ParamType::Enum {
                name: "".to_string(),
                enum_variants: deeper_enum_variants.clone(),
                generics: vec![],
            },
            ParamType::Bool,
        ]);
        let struct_a_type = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![],
        };

        let struct_a_token = Token::Struct(vec![
            Token::Enum(Box::new((1, deeper_enum_token, deeper_enum_variants))),
            Token::U32(11332),
        ]);

        /*
         enum TopLevelEnum {
            v1: StructA,
            v2: bool,
            v3: u64
        }
        */

        let types = to_named(&[struct_a_type, ParamType::Bool, ParamType::U64]);
        let top_level_enum_variants = EnumVariants::new(types)?;
        let top_level_enum_token =
            Token::Enum(Box::new((0, struct_a_token, top_level_enum_variants)));

        let result = ABIEncoder::default().encode(slice::from_ref(&top_level_enum_token))?;

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 0, // TopLevelEnum::v1 discriminant
            0, 0, 0, 0, 0, 0, 0, 1, // DeeperEnum::v2 discriminant
            48, 49, 50, 51, 52, 53, 54, 55, 56, 57, // str[10]
            0, 0, 44, 68, // StructA.some_number
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_nested_structs() -> Result<()> {
        let token = Token::Struct(vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ]);

        let result = ABIEncoder::default().encode(&[token])?;

        let expected = [
            0, 10, // u16
            1,  // bool
            1, 2, // [u8, u8]
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encode_comprehensive() -> Result<()> {
        let foo = Token::Struct(vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ]);
        let arr_u8 = Token::Array(vec![Token::U8(1), Token::U8(2)]);
        let b256 = Token::B256([255; 32]);
        let str_arr = Token::StringArray(StaticStringToken::new(
            "This is a full sentence".into(),
            Some(23),
        ));
        let tokens = vec![foo, arr_u8, b256, str_arr];

        let result = ABIEncoder::default().encode(&tokens)?;

        let expected = [
            0, 10, // foo.x == 10u16
            1,  // foo.y.a == true
            1,  // foo.y.b.0 == 1u8
            2,  // foo.y.b.1 == 2u8
            1,  // u8[2].0 == 1u8
            2,  // u8[2].0 == 2u8
            255, 255, 255, 255, 255, 255, 255, 255, // b256
            255, 255, 255, 255, 255, 255, 255, 255, // b256
            255, 255, 255, 255, 255, 255, 255, 255, // b256
            255, 255, 255, 255, 255, 255, 255, 255, // b256
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, // str[23]
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn enums_with_only_unit_variants_are_encoded_in_one_word() -> Result<()> {
        let expected = [0, 0, 0, 0, 0, 0, 0, 1];

        let types = to_named(&[ParamType::Unit, ParamType::Unit]);
        let enum_selector = Box::new((1, Token::Unit, EnumVariants::new(types)?));

        let actual = ABIEncoder::default().encode(&[Token::Enum(enum_selector)])?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn vec_in_enum() -> Result<()> {
        // arrange
        let types = to_named(&[ParamType::B256, ParamType::Vector(Box::new(ParamType::U64))]);
        let variants = EnumVariants::new(types)?;
        let selector = (1, Token::Vector(vec![Token::U64(5)]), variants);
        let token = Token::Enum(Box::new(selector));

        // act
        let result = ABIEncoder::default().encode(&[token])?;

        // assert
        let expected = [
            0, 0, 0, 0, 0, 0, 0, 1, // enum dicsriminant
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 5, // vec[len, u64]
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn enum_in_vec() -> Result<()> {
        // arrange
        let types = to_named(&[ParamType::B256, ParamType::U8]);
        let variants = EnumVariants::new(types)?;
        let selector = (1, Token::U8(8), variants);
        let enum_token = Token::Enum(Box::new(selector));

        let vec_token = Token::Vector(vec![enum_token]);

        // act
        let result = ABIEncoder::default().encode(&[vec_token])?;

        // assert
        let expected = [
            0, 0, 0, 0, 0, 0, 0, 1, // vec len
            0, 0, 0, 0, 0, 0, 0, 1, 8, // enum discriminant and u8 value
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn vec_in_struct() -> Result<()> {
        // arrange
        let token = Token::Struct(vec![Token::Vector(vec![Token::U64(5)]), Token::U8(9)]);

        // act
        let result = ABIEncoder::default().encode(&[token])?;

        // assert
        let expected = [
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 5, // vec[len, u64]
            9, // u8
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn vec_in_vec() -> Result<()> {
        // arrange
        let token = Token::Vector(vec![Token::Vector(vec![Token::U8(5), Token::U8(6)])]);

        // act
        let result = ABIEncoder::default().encode(&[token])?;

        // assert
        let expected = [
            0, 0, 0, 0, 0, 0, 0, 1, // vec1 len
            0, 0, 0, 0, 0, 0, 0, 2, 5, 6, // vec2 [len, u8, u8]
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn max_depth_surpassed() {
        const MAX_DEPTH: usize = 2;
        let config = EncoderConfig {
            max_depth: MAX_DEPTH,
            ..Default::default()
        };
        let msg = "depth limit `2` reached while encoding. Try increasing it".to_string();

        [nested_struct, nested_enum, nested_tuple, nested_array]
            .iter()
            .map(|fun| fun(MAX_DEPTH + 1))
            .for_each(|token| {
                assert_encoding_failed(config, token, &msg);
            });
    }

    fn assert_encoding_failed(config: EncoderConfig, token: Token, msg: &str) {
        let encoder = ABIEncoder::new(config);

        let err = encoder.encode(&[token]);

        let Err(Error::Codec(actual_msg)) = err else {
            panic!("expected a Codec error. Got: `{err:?}`");
        };
        assert_eq!(actual_msg, msg);
    }

    fn nested_struct(depth: usize) -> Token {
        let fields = if depth == 1 {
            vec![Token::U8(255), Token::String("bloopblip".to_string())]
        } else {
            vec![nested_struct(depth - 1)]
        };

        Token::Struct(fields)
    }

    fn nested_enum(depth: usize) -> Token {
        if depth == 0 {
            return Token::U8(255);
        }

        let inner_enum = nested_enum(depth - 1);

        // Create a basic EnumSelector for the current level (the `EnumVariants` is not
        // actually accurate but it's not used for encoding)
        let selector = (
            0u64,
            inner_enum,
            EnumVariants::new(to_named(&[ParamType::U64])).unwrap(),
        );

        Token::Enum(Box::new(selector))
    }

    fn nested_array(depth: usize) -> Token {
        if depth == 1 {
            Token::Array(vec![Token::U8(255)])
        } else {
            Token::Array(vec![nested_array(depth - 1)])
        }
    }

    fn nested_tuple(depth: usize) -> Token {
        let fields = if depth == 1 {
            vec![Token::U8(255), Token::String("bloopblip".to_string())]
        } else {
            vec![nested_tuple(depth - 1)]
        };

        Token::Tuple(fields)
    }
}
