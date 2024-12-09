mod bounded_decoder;
mod decode_as_debug_str;

use crate::{
    codec::abi_decoder::{
        bounded_decoder::BoundedDecoder, decode_as_debug_str::decode_as_debug_str,
    },
    types::{errors::Result, param_types::ParamType, Token},
};

#[derive(Debug, Clone, Copy)]
pub struct DecoderConfig {
    /// Entering a struct, array, tuple, enum or vector increases the depth. Decoding will fail if
    /// the current depth becomes greater than `max_depth` configured here.
    pub max_depth: usize,
    /// Every decoded Token will increase the token count. Decoding will fail if the current
    /// token count becomes greater than `max_tokens` configured here.
    pub max_tokens: usize,
}

// ANCHOR: default_decoder_config
impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            max_depth: 45,
            max_tokens: 10_000,
        }
    }
}
// ANCHOR_END: default_decoder_config

#[derive(Default)]
pub struct ABIDecoder {
    pub config: DecoderConfig,
}

impl ABIDecoder {
    pub fn new(config: DecoderConfig) -> Self {
        Self { config }
    }

    /// Decodes `bytes` following the schema described in `param_type` into its respective `Token`.
    ///
    /// # Arguments
    ///
    /// * `param_type`: The `ParamType` of the type we expect is encoded
    ///                  inside `bytes`.
    /// * `bytes`:       The bytes to be used in the decoding process.
    /// # Examples
    ///
    /// ```
    /// use fuels_core::codec::ABIDecoder;
    /// use fuels_core::traits::Tokenizable;
    /// use fuels_core::types::param_types::ParamType;
    ///
    /// let decoder = ABIDecoder::default();
    ///
    /// let token = decoder.decode(&ParamType::U64,  &[0, 0, 0, 0, 0, 0, 0, 7]).unwrap();
    ///
    /// assert_eq!(u64::from_token(token).unwrap(), 7u64);
    /// ```
    pub fn decode(&self, param_type: &ParamType, bytes: &[u8]) -> Result<Token> {
        BoundedDecoder::new(self.config).decode(param_type, bytes)
    }

    /// Same as `decode` but decodes multiple `ParamType`s in one go.
    /// # Examples
    /// ```
    /// use fuels_core::codec::ABIDecoder;
    /// use fuels_core::types::param_types::ParamType;
    /// use fuels_core::types::Token;
    ///
    /// let decoder = ABIDecoder::default();
    /// let data: &[u8] = &[7, 8];
    ///
    /// let tokens = decoder.decode_multiple(&[ParamType::U8, ParamType::U8], &data).unwrap();
    ///
    /// assert_eq!(tokens, vec![Token::U8(7), Token::U8(8)]);
    /// ```
    pub fn decode_multiple(&self, param_types: &[ParamType], bytes: &[u8]) -> Result<Vec<Token>> {
        BoundedDecoder::new(self.config).decode_multiple(param_types, bytes)
    }

    /// Decodes `bytes` following the schema described in `param_type` into its respective debug
    /// string.
    ///
    /// # Arguments
    ///
    /// * `param_type`: The `ParamType` of the type we expect is encoded
    ///                  inside `bytes`.
    /// * `bytes`:       The bytes to be used in the decoding process.
    /// # Examples
    ///
    /// ```
    /// use fuels_core::codec::ABIDecoder;
    /// use fuels_core::types::param_types::ParamType;
    ///
    /// let decoder = ABIDecoder::default();
    ///
    /// let debug_string = decoder.decode_as_debug_str(&ParamType::U64,  &[0, 0, 0, 0, 0, 0, 0, 7]).unwrap();
    /// let expected_value = 7u64;
    ///
    /// assert_eq!(debug_string, format!("{expected_value}"));
    /// ```
    pub fn decode_as_debug_str(&self, param_type: &ParamType, bytes: &[u8]) -> Result<String> {
        let token = BoundedDecoder::new(self.config).decode(param_type, bytes)?;
        decode_as_debug_str(param_type, &token)
    }

    pub fn decode_multiple_as_debug_str(
        &self,
        param_types: &[ParamType],
        bytes: &[u8],
    ) -> Result<Vec<String>> {
        let token = BoundedDecoder::new(self.config).decode_multiple(param_types, bytes)?;
        token
            .into_iter()
            .zip(param_types)
            .map(|(token, param_type)| decode_as_debug_str(param_type, &token))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use ParamType::*;

    use super::*;
    use crate::{
        constants::WORD_SIZE,
        to_named,
        traits::Parameterize,
        types::{errors::Error, param_types::EnumVariants, StaticStringToken, U256},
    };

    #[test]
    fn decode_multiple_uint() -> Result<()> {
        let types = vec![
            ParamType::U8,
            ParamType::U16,
            ParamType::U32,
            ParamType::U64,
            ParamType::U128,
            ParamType::U256,
        ];

        let data = [
            255, // u8
            255, 255, // u16
            255, 255, 255, 255, // u32
            255, 255, 255, 255, 255, 255, 255, 255, // u64
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, // u128
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, // u256
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![
            Token::U8(u8::MAX),
            Token::U16(u16::MAX),
            Token::U32(u32::MAX),
            Token::U64(u64::MAX),
            Token::U128(u128::MAX),
            Token::U256(U256::MAX),
        ];
        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_bool() -> Result<()> {
        let types = vec![ParamType::Bool, ParamType::Bool];
        let data = [1, 0];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::Bool(true), Token::Bool(false)];

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_b256() -> Result<()> {
        let data = [
            213, 87, 156, 70, 223, 204, 127, 24, 32, 112, 19, 230, 91, 68, 228, 203, 78, 44, 34,
            152, 244, 172, 69, 123, 168, 248, 39, 67, 243, 30, 147, 11,
        ];

        let decoded = ABIDecoder::default().decode(&ParamType::B256, &data)?;

        assert_eq!(decoded, Token::B256(data));

        Ok(())
    }

    #[test]
    fn decode_string_array() -> Result<()> {
        let types = vec![ParamType::StringArray(23), ParamType::StringArray(5)];
        let data = [
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
            72, 101, 108, 108, 111, // Hello
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![
            Token::StringArray(StaticStringToken::new(
                "This is a full sentence".into(),
                Some(23),
            )),
            Token::StringArray(StaticStringToken::new("Hello".into(), Some(5))),
        ];

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_string_slice() -> Result<()> {
        let data = [
            0, 0, 0, 0, 0, 0, 0, 23, // [length]
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
        ];

        let decoded = ABIDecoder::default().decode(&ParamType::StringSlice, &data)?;

        let expected = Token::StringSlice(StaticStringToken::new(
            "This is a full sentence".into(),
            None,
        ));

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_string() -> Result<()> {
        let data = [
            0, 0, 0, 0, 0, 0, 0, 23, // [length]
            84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 102, 117, 108, 108, 32, 115, 101, 110,
            116, 101, 110, 99, 101, //This is a full sentence
        ];

        let decoded = ABIDecoder::default().decode(&ParamType::String, &data)?;

        let expected = Token::String("This is a full sentence".to_string());

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_tuple() -> Result<()> {
        let param_type = ParamType::Tuple(vec![ParamType::U32, ParamType::Bool]);
        let data = [
            0, 0, 0, 255, //u32
            1,   //bool
        ];

        let result = ABIDecoder::default().decode(&param_type, &data)?;

        let expected = Token::Tuple(vec![Token::U32(255), Token::Bool(true)]);

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn decode_array() -> Result<()> {
        let types = vec![ParamType::Array(Box::new(ParamType::U8), 2)];
        let data = [255, 42];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::Array(vec![Token::U8(255), Token::U8(42)])];
        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_struct() -> Result<()> {
        // struct MyStruct {
        //     foo: u8,
        //     bar: bool,
        // }

        let data = [1, 1];

        let param_type = ParamType::Struct {
            name: "".to_string(),
            fields: to_named(&[ParamType::U8, ParamType::Bool]),
            generics: vec![],
        };

        let decoded = ABIDecoder::default().decode(&param_type, &data)?;

        let expected = Token::Struct(vec![Token::U8(1), Token::Bool(true)]);

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_bytes() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 7, 255, 0, 1, 2, 3, 4, 5];

        let decoded = ABIDecoder::default().decode(&ParamType::Bytes, &data)?;

        let expected = Token::Bytes([255, 0, 1, 2, 3, 4, 5].to_vec());

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_raw_slice() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 7, 255, 0, 1, 2, 3, 4, 5];

        let decoded = ABIDecoder::default().decode(&ParamType::RawSlice, &data)?;

        let expected = Token::RawSlice([255, 0, 1, 2, 3, 4, 5].to_vec());

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_enum() -> Result<()> {
        // enum MyEnum {
        //     x: u32,
        //     y: bool,
        // }

        let types = to_named(&[ParamType::U32, ParamType::Bool]);
        let inner_enum_types = EnumVariants::new(types)?;
        let types = vec![ParamType::Enum {
            name: "".to_string(),
            enum_variants: inner_enum_types.clone(),
            generics: vec![],
        }];

        let data = [
            0, 0, 0, 0, 0, 0, 0, 0, // discriminant
            0, 0, 0, 42, // u32
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::Enum(Box::new((0, Token::U32(42), inner_enum_types)))];
        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn decode_nested_struct() -> Result<()> {
        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        let fields = to_named(&[
            ParamType::U16,
            ParamType::Struct {
                name: "".to_string(),
                fields: to_named(&[
                    ParamType::Bool,
                    ParamType::Array(Box::new(ParamType::U8), 2),
                ]),
                generics: vec![],
            },
        ]);
        let nested_struct = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![],
        };

        let data = [0, 10, 1, 1, 2];

        let decoded = ABIDecoder::default().decode(&nested_struct, &data)?;

        let my_nested_struct = vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ];

        assert_eq!(decoded, Token::Struct(my_nested_struct));

        Ok(())
    }

    #[test]
    fn decode_comprehensive() -> Result<()> {
        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        // fn: long_function(Foo,u8[2],b256,str[3],str)

        // Parameters
        let fields = to_named(&[
            ParamType::U16,
            ParamType::Struct {
                name: "".to_string(),
                fields: to_named(&[
                    ParamType::Bool,
                    ParamType::Array(Box::new(ParamType::U8), 2),
                ]),
                generics: vec![],
            },
        ]);
        let nested_struct = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![],
        };

        let u8_arr = ParamType::Array(Box::new(ParamType::U8), 2);
        let b256 = ParamType::B256;

        let types = [nested_struct, u8_arr, b256];

        let bytes = [
            0, 10, // u16
            1,  // bool
            1, 2, // array[u8;2]
            1, 2, // array[u8;2]
            213, 87, 156, 70, 223, 204, 127, 24, 32, 112, 19, 230, 91, 68, 228, 203, 78, 44, 34,
            152, 244, 172, 69, 123, 168, 248, 39, 67, 243, 30, 147, 11, // b256
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &bytes)?;

        // Expected tokens
        let foo = Token::Struct(vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ]);

        let u8_arr = Token::Array(vec![Token::U8(1), Token::U8(2)]);

        let b256 = Token::B256([
            213, 87, 156, 70, 223, 204, 127, 24, 32, 112, 19, 230, 91, 68, 228, 203, 78, 44, 34,
            152, 244, 172, 69, 123, 168, 248, 39, 67, 243, 30, 147, 11,
        ]);

        let expected: Vec<Token> = vec![foo, u8_arr, b256];

        assert_eq!(decoded, expected);

        Ok(())
    }

    #[test]
    fn enums_with_all_unit_variants_are_decoded_from_one_word() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1];
        let types = to_named(&[ParamType::Unit, ParamType::Unit]);
        let enum_variants = EnumVariants::new(types)?;
        let enum_w_only_units = ParamType::Enum {
            name: "".to_string(),
            enum_variants: enum_variants.clone(),
            generics: vec![],
        };

        let result = ABIDecoder::default().decode(&enum_w_only_units, &data)?;

        let expected_enum = Token::Enum(Box::new((1, Token::Unit, enum_variants)));
        assert_eq!(result, expected_enum);

        Ok(())
    }

    #[test]
    fn out_of_bounds_discriminant_is_detected() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let types = to_named(&[ParamType::U64]);
        let enum_variants = EnumVariants::new(types)?;
        let enum_type = ParamType::Enum {
            name: "".to_string(),
            enum_variants,
            generics: vec![],
        };

        let result = ABIDecoder::default().decode(&enum_type, &data);

        let error = result.expect_err("should have resulted in an error");

        let expected_msg = "discriminant `1` doesn't point to any variant: ";
        assert!(matches!(error, Error::Other(str) if str.starts_with(expected_msg)));

        Ok(())
    }

    #[test]
    pub fn division_by_zero() {
        let param_type = Vec::<[u16; 0]>::param_type();
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    pub fn multiply_overflow_enum() {
        let result = ABIDecoder::default().decode(
            &Enum {
                name: "".to_string(),
                enum_variants: EnumVariants::new(to_named(&[
                    Array(Box::new(Array(Box::new(RawSlice), 8)), usize::MAX),
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                    B256,
                ]))
                .unwrap(),
                generics: vec![U16],
            },
            &[],
        );

        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    pub fn multiply_overflow_arith() {
        let mut param_type: ParamType = U16;
        for _ in 0..50 {
            param_type = Array(Box::new(param_type), 8);
        }
        let result = ABIDecoder::default().decode(
            &Enum {
                name: "".to_string(),
                enum_variants: EnumVariants::new(to_named(&[param_type])).unwrap(),
                generics: vec![U16],
            },
            &[],
        );
        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    pub fn capacity_overflow() {
        let result = ABIDecoder::default().decode(
            &Array(Box::new(Array(Box::new(Tuple(vec![])), usize::MAX)), 1),
            &[],
        );
        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    pub fn stack_overflow() {
        let mut param_type: ParamType = U16;
        for _ in 0..13500 {
            param_type = Vector(Box::new(param_type));
        }
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    pub fn capacity_malloc() {
        let param_type = Array(Box::new(U8), usize::MAX);
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::Codec(_))));
    }

    #[test]
    fn max_depth_surpassed() {
        const MAX_DEPTH: usize = 2;
        let config = DecoderConfig {
            max_depth: MAX_DEPTH,
            ..Default::default()
        };
        let msg = format!("depth limit `{MAX_DEPTH}` reached while decoding. Try increasing it");
        // for each nested enum so that it may read the discriminant
        let data = [0; MAX_DEPTH * WORD_SIZE];

        [nested_struct, nested_enum, nested_tuple, nested_array]
            .iter()
            .map(|fun| fun(MAX_DEPTH + 1))
            .for_each(|param_type| {
                assert_decoding_failed_w_data(config, &param_type, &msg, &data);
            })
    }

    #[test]
    fn depth_is_not_reached() {
        const MAX_DEPTH: usize = 3;
        const ACTUAL_DEPTH: usize = MAX_DEPTH - 1;

        // enough data to decode 2*ACTUAL_DEPTH enums (discriminant + u8 = 2*WORD_SIZE)
        let data = [0; 2 * ACTUAL_DEPTH * (WORD_SIZE * 2)];
        let config = DecoderConfig {
            max_depth: MAX_DEPTH,
            ..Default::default()
        };

        [nested_struct, nested_enum, nested_tuple, nested_array]
            .into_iter()
            .map(|fun| fun(ACTUAL_DEPTH))
            .map(|param_type| {
                // Wrapping everything in a structure so that we may check whether the depth is
                // decremented after finishing every struct field.
                ParamType::Struct {
                    name: "".to_string(),
                    fields: to_named(&[param_type.clone(), param_type]),
                    generics: vec![],
                }
            })
            .for_each(|param_type| {
                ABIDecoder::new(config).decode(&param_type, &data).unwrap();
            })
    }

    #[test]
    fn too_many_tokens() {
        let config = DecoderConfig {
            max_tokens: 3,
            ..Default::default()
        };
        {
            let data = [0; 3 * WORD_SIZE];
            let inner_param_types = vec![ParamType::U64; 3];
            for param_type in [
                ParamType::Struct {
                    name: "".to_string(),
                    fields: to_named(&inner_param_types),
                    generics: vec![],
                },
                ParamType::Tuple(inner_param_types.clone()),
                ParamType::Array(Box::new(ParamType::U64), 3),
            ] {
                assert_decoding_failed_w_data(
                    config,
                    &param_type,
                    "token limit `3` reached while decoding. Try increasing it",
                    &data,
                );
            }
        }
        {
            let data = [0, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3];

            assert_decoding_failed_w_data(
                config,
                &ParamType::Vector(Box::new(ParamType::U8)),
                "token limit `3` reached while decoding. Try increasing it",
                &data,
            );
        }
    }

    #[test]
    fn token_count_is_being_reset_between_decodings() {
        // given
        let config = DecoderConfig {
            max_tokens: 3,
            ..Default::default()
        };

        let param_type = ParamType::Array(Box::new(ParamType::StringArray(0)), 2);

        let decoder = ABIDecoder::new(config);
        decoder.decode(&param_type, &[]).unwrap();

        // when
        let result = decoder.decode(&param_type, &[]);

        // then
        result.expect("element count to be reset");
    }

    fn assert_decoding_failed_w_data(
        config: DecoderConfig,
        param_type: &ParamType,
        msg: &str,
        data: &[u8],
    ) {
        let decoder = ABIDecoder::new(config);

        let err = decoder.decode(param_type, data);

        let Err(Error::Codec(actual_msg)) = err else {
            panic!("expected a `Codec` error. Got: `{err:?}`");
        };

        assert_eq!(actual_msg, msg);
    }

    fn nested_struct(depth: usize) -> ParamType {
        let fields = if depth == 1 {
            vec![]
        } else {
            to_named(&[nested_struct(depth - 1)])
        };

        ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![],
        }
    }

    fn nested_enum(depth: usize) -> ParamType {
        let fields = if depth == 1 {
            to_named(&[ParamType::U8])
        } else {
            to_named(&[nested_enum(depth - 1)])
        };

        ParamType::Enum {
            name: "".to_string(),
            enum_variants: EnumVariants::new(fields).unwrap(),
            generics: vec![],
        }
    }

    fn nested_array(depth: usize) -> ParamType {
        let field = if depth == 1 {
            ParamType::U8
        } else {
            nested_array(depth - 1)
        };

        ParamType::Array(Box::new(field), 1)
    }

    fn nested_tuple(depth: usize) -> ParamType {
        let fields = if depth == 1 {
            vec![ParamType::U8]
        } else {
            vec![nested_tuple(depth - 1)]
        };

        ParamType::Tuple(fields)
    }
}
