mod bounded_decoder;

use crate::{
    codec::abi_decoder::bounded_decoder::BoundedDecoder,
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

    /// Decode data from one of the receipt returns.
    pub fn decode_receipt_return(&self, param_type: &ParamType, bytes: &[u8]) -> Result<Token> {
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
}

#[cfg(test)]
mod tests {
    use std::vec;

    use ParamType::*;

    use super::*;
    use crate::{
        constants::WORD_SIZE,
        traits::Parameterize,
        types::{enum_variants::EnumVariants, errors::Error, StaticStringToken, U256},
    };

    #[test]
    fn decode_int() -> Result<()> {
        let data = [0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff];

        let decoded = ABIDecoder::default().decode(&ParamType::U32, &data)?;

        assert_eq!(decoded, Token::U32(u32::MAX));
        Ok(())
    }

    #[test]
    fn decode_multiple_int() -> Result<()> {
        let types = vec![
            ParamType::U32,
            ParamType::U8,
            ParamType::U16,
            ParamType::U64,
            ParamType::U128,
            ParamType::U256,
        ];
        let data = [
            0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff, // u32
            0xff, // u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xff, 0xff, // u16
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // u64
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, // u128
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, // u256
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![
            Token::U32(u32::MAX),
            Token::U8(u8::MAX),
            Token::U16(u16::MAX),
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
        let data = [0x01, 0x0];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::Bool(true), Token::Bool(false)];

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_b256() -> Result<()> {
        let data = [
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, 0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44,
            0xe4, 0xcb, 0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, 0xa8, 0xf8, 0x27, 0x43,
            0xf3, 0x1e, 0x93, 0xb,
        ];

        let decoded = ABIDecoder::default().decode(&ParamType::B256, &data)?;

        assert_eq!(decoded, Token::B256(data));
        Ok(())
    }

    #[test]
    fn decode_string_array() -> Result<()> {
        let types = vec![ParamType::StringArray(23), ParamType::StringArray(5)];
        let data = [
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // This is
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // a full s
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x00, // entence
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x00, 0x00, // Hello
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
        let types = vec![ParamType::StringSlice];
        let data = [
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // This is
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // a full s
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, // entence
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::StringSlice(StaticStringToken::new(
            "This is a full sentence".into(),
            None,
        ))];

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_array() -> Result<()> {
        // Create a parameter type for u8[2].
        let types = vec![ParamType::Array(Box::new(ParamType::U8), 2)];
        let data = [0xff, 0x2a];

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

        let data = [
            0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];
        let param_type = ParamType::Struct {
            fields: vec![ParamType::U8, ParamType::Bool],
            generics: vec![],
        };

        let decoded = ABIDecoder::default().decode(&param_type, &data)?;

        let expected = Token::Struct(vec![Token::U8(1), Token::Bool(true)]);

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_bytes() -> Result<()> {
        let data = [0xFF, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let decoded = ABIDecoder::default().decode(&ParamType::Bytes, &data)?;

        let expected = Token::Bytes(data.to_vec());

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_enum() -> Result<()> {
        // enum MyEnum {
        //     x: u32,
        //     y: bool,
        // }

        let types = vec![ParamType::U32, ParamType::Bool];
        let inner_enum_types = EnumVariants::new(types)?;
        let types = vec![ParamType::Enum {
            variants: inner_enum_types.clone(),
            generics: vec![],
        }];

        // "0" discriminant and 42 enum value
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2a,
        ];

        let decoded = ABIDecoder::default().decode_multiple(&types, &data)?;

        let expected = vec![Token::Enum(Box::new((0, Token::U32(42), inner_enum_types)))];
        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decoder_will_skip_enum_padding_and_decode_next_arg() -> Result<()> {
        // struct MyStruct {
        //     par1: MyEnum,
        //     par2: u32
        // }

        // enum MyEnum {
        //     x: b256,
        //     y: u32,
        // }

        let types = vec![ParamType::B256, ParamType::U32];
        let inner_enum_types = EnumVariants::new(types)?;

        let fields = vec![
            ParamType::Enum {
                variants: inner_enum_types.clone(),
                generics: vec![],
            },
            ParamType::U32,
        ];
        let struct_type = ParamType::Struct {
            fields,
            generics: vec![],
        };

        let enum_discriminant_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1];
        let enum_data_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x30, 0x39];
        // this padding is due to the biggest variant of MyEnum being 3 WORDs bigger than the chosen variant
        let enum_padding_enc = vec![0x0; 3 * WORD_SIZE];
        let struct_par2_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xD4, 0x31];
        let data: Vec<u8> = vec![
            enum_discriminant_enc,
            enum_padding_enc,
            enum_data_enc,
            struct_par2_enc,
        ]
        .into_iter()
        .flatten()
        .collect();

        let decoded = ABIDecoder::default().decode(&struct_type, &data)?;

        let expected = Token::Struct(vec![
            Token::Enum(Box::new((1, Token::U32(12345), inner_enum_types))),
            Token::U32(54321),
        ]);
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

        let fields = vec![
            ParamType::U16,
            ParamType::Struct {
                fields: vec![
                    ParamType::Bool,
                    ParamType::Array(Box::new(ParamType::U8), 2),
                ],
                generics: vec![],
            },
        ];
        let nested_struct = ParamType::Struct {
            fields,
            generics: vec![],
        };

        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
            0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];

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
        let fields = vec![
            ParamType::U16,
            ParamType::Struct {
                fields: vec![
                    ParamType::Bool,
                    ParamType::Array(Box::new(ParamType::U8), 2),
                ],
                generics: vec![],
            },
        ];
        let nested_struct = ParamType::Struct {
            fields,
            generics: vec![],
        };

        let u8_arr = ParamType::Array(Box::new(ParamType::U8), 2);
        let b256 = ParamType::B256;

        let types = [nested_struct, u8_arr, b256];

        let bytes = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, // u16
            0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // bool
            0x1, 0x2, // array[u8]
            0x1, 0x2, // array[u8]
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, // b256 start
            0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44, 0xe4, 0xcb, //
            0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, //
            0xa8, 0xf8, 0x27, 0x43, 0xf3, 0x1e, 0x93,
            0xb, // b256 end
                 // 0x66, 0x6f, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, // "foo"
                 // 0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, //
                 // 0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, //
                 // 0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, //
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
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, 0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44,
            0xe4, 0xcb, 0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, 0xa8, 0xf8, 0x27, 0x43,
            0xf3, 0x1e, 0x93, 0xb,
        ]);

        let expected: Vec<Token> = vec![foo, u8_arr, b256];

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn units_in_structs_are_decoded_as_one_word() -> Result<()> {
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let struct_type = ParamType::Struct {
            fields: vec![ParamType::Unit, ParamType::U64],
            generics: vec![],
        };

        let actual = ABIDecoder::default().decode(&struct_type, &data)?;

        let expected = Token::Struct(vec![Token::Unit, Token::U64(u64::MAX)]);
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn enums_with_all_unit_variants_are_decoded_from_one_word() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1];
        let types = vec![ParamType::Unit, ParamType::Unit];
        let variants = EnumVariants::new(types)?;
        let enum_w_only_units = ParamType::Enum {
            variants: variants.clone(),
            generics: vec![],
        };

        let result = ABIDecoder::default().decode(&enum_w_only_units, &data)?;

        let expected_enum = Token::Enum(Box::new((1, Token::Unit, variants)));
        assert_eq!(result, expected_enum);
        Ok(())
    }

    #[test]
    fn out_of_bounds_discriminant_is_detected() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let types = vec![ParamType::U32];
        let variants = EnumVariants::new(types)?;
        let enum_type = ParamType::Enum {
            variants,
            generics: vec![],
        };

        let result = ABIDecoder::default().decode(&enum_type, &data);

        let error = result.expect_err("Should have resulted in an error");

        let expected_msg = "Discriminant '1' doesn't point to any variant: ";
        assert!(matches!(error, Error::InvalidData(str) if str.starts_with(expected_msg)));
        Ok(())
    }

    #[test]
    pub fn division_by_zero() {
        let param_type = Vec::<[u16; 0]>::param_type();
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    pub fn multiply_overflow_enum() {
        let result = ABIDecoder::default().decode(
            &Enum {
                variants: EnumVariants::new(vec![
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
                ])
                .unwrap(),
                generics: vec![U16],
            },
            &[],
        );
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    pub fn multiply_overflow_arith() {
        let mut param_type: ParamType = U16;
        for _ in 0..50 {
            param_type = Array(Box::new(param_type), 8);
        }
        let result = ABIDecoder::default().decode(
            &Enum {
                variants: EnumVariants::new(vec![param_type]).unwrap(),
                generics: vec![U16],
            },
            &[],
        );
        assert!(matches!(result, Err(Error::InvalidData(_))));
    }

    #[test]
    pub fn capacity_overflow() {
        let result = ABIDecoder::default().decode(
            &Array(Box::new(Array(Box::new(Tuple(vec![])), usize::MAX)), 1),
            &[],
        );
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    pub fn stack_overflow() {
        let mut param_type: ParamType = U16;
        for _ in 0..13500 {
            param_type = Vector(Box::new(param_type));
        }
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::InvalidType(_))));
    }

    #[test]
    pub fn capacity_maloc() {
        let param_type = Array(Box::new(U8), usize::MAX);
        let result = ABIDecoder::default().decode(&param_type, &[]);
        assert!(matches!(result, Err(Error::InvalidData(_))));
    }

    #[test]
    fn decoding_enum_with_more_than_one_heap_type_variant_fails() -> Result<()> {
        let mut param_types = vec![
            ParamType::U64,
            ParamType::Bool,
            ParamType::Vector(Box::from(ParamType::U64)),
        ];
        // empty data
        let data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let variants = EnumVariants::new(param_types.clone())?;
        let enum_param_type = ParamType::Enum {
            variants,
            generics: vec![],
        };
        // it works if there is only one heap type
        let _ = ABIDecoder::default().decode(&enum_param_type, &data)?;

        param_types.append(&mut vec![ParamType::Bytes]);
        let variants = EnumVariants::new(param_types)?;
        let enum_param_type = ParamType::Enum {
            variants,
            generics: vec![],
        };
        // fails if there is more than one variant using heap type in the enum
        let error = ABIDecoder::default()
            .decode(&enum_param_type, &data)
            .expect_err("Should fail");
        let expected_error =
            "Invalid type: Enums currently support only one heap-type variant. Found: 2"
                .to_string();
        assert_eq!(error.to_string(), expected_error);

        Ok(())
    }

    #[test]
    fn enums_w_too_deeply_nested_heap_types_not_allowed() {
        let param_types = vec![
            ParamType::U8,
            ParamType::Struct {
                fields: vec![ParamType::RawSlice],
                generics: vec![],
            },
        ];
        let variants = EnumVariants::new(param_types).unwrap();
        let enum_param_type = ParamType::Enum {
            variants,
            generics: vec![],
        };

        let err = ABIDecoder::default()
            .decode(&enum_param_type, &[])
            .expect_err("should have failed");

        let Error::InvalidType(msg) = err else {
            panic!("Unexpected err: {err}");
        };

        assert_eq!(
            msg,
            "Enums currently support only one level deep heap types."
        );
    }

    #[test]
    fn max_depth_surpassed() {
        const MAX_DEPTH: usize = 2;
        let config = DecoderConfig {
            max_depth: MAX_DEPTH,
            ..Default::default()
        };
        let msg = format!("Depth limit ({MAX_DEPTH}) reached while decoding. Try increasing it.");
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
                    fields: vec![param_type.clone(), param_type],
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

        let data = [0; 3 * WORD_SIZE];
        let el = ParamType::U8;
        for param_type in [
            ParamType::Struct {
                fields: vec![el.clone(); 3],
                generics: vec![],
            },
            ParamType::Tuple(vec![el.clone(); 3]),
            ParamType::Array(Box::new(el.clone()), 3),
            ParamType::Vector(Box::new(el)),
        ] {
            assert_decoding_failed_w_data(
                config,
                &param_type,
                "Token limit (3) reached while decoding. Try increasing it.",
                &data,
            );
        }
    }

    #[test]
    fn vectors_of_zst_are_not_supported() {
        let param_type = ParamType::Vector(Box::new(ParamType::StringArray(0)));

        let err = ABIDecoder::default()
            .decode(&param_type, &[])
            .expect_err("Vectors of ZST should be prohibited");

        let Error::InvalidType(msg) = err else {
            panic!("Expected error of type InvalidType")
        };
        assert_eq!(
            msg,
            "Cannot calculate the number of elements because the type is zero-sized."
        );
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
        result.expect("Element count to be reset");
    }

    fn assert_decoding_failed_w_data(
        config: DecoderConfig,
        param_type: &ParamType,
        msg: &str,
        data: &[u8],
    ) {
        let decoder = ABIDecoder::new(config);

        let err = decoder.decode(param_type, data);

        let Err(Error::InvalidType(actual_msg)) = err else {
            panic!("Unexpected an InvalidType error! Got: {err:?}");
        };
        assert_eq!(actual_msg, msg);
    }

    fn nested_struct(depth: usize) -> ParamType {
        let fields = if depth == 1 {
            vec![]
        } else {
            vec![nested_struct(depth - 1)]
        };

        ParamType::Struct {
            fields,
            generics: vec![],
        }
    }

    fn nested_enum(depth: usize) -> ParamType {
        let fields = if depth == 1 {
            vec![ParamType::U8]
        } else {
            vec![nested_enum(depth - 1)]
        };

        ParamType::Enum {
            variants: EnumVariants::new(fields).unwrap(),
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
