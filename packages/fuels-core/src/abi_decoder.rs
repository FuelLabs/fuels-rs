use std::{convert::TryInto, str};

use fuel_types::bytes::padded_len_usize;
use fuels_types::{
    constants::WORD_SIZE,
    enum_variants::EnumVariants,
    errors::{error, Error, Result},
    param_types::ParamType,
    unzip_param_types, StringToken, Token,
};

use crate::Tokenizable;

#[derive(Debug, Clone)]
struct DecodeResult {
    token: Token,
    bytes_read: usize,
}

pub struct ABIDecoder;

impl ABIDecoder {
    /// Decodes types described by `param_types` into their respective `Token`s
    /// using the data in `bytes` and `receipts`.
    ///
    /// # Arguments
    ///
    /// * `param_types`: The ParamType's of the types we expect are encoded
    ///                  inside `bytes` and `receipts`.
    /// * `bytes`:       The bytes to be used in the decoding process.
    /// # Examples
    ///
    /// ```
    /// use fuels_core::abi_decoder::ABIDecoder;
    /// use fuels_types::{Token, param_types::ParamType};
    ///
    /// let tokens = ABIDecoder::decode(&[ParamType::U8, ParamType::U8], &[0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,2]).unwrap();
    ///
    /// assert_eq!(tokens, vec![Token::U8(1), Token::U8(2)])
    /// ```
    pub fn decode(param_types: &[ParamType], bytes: &[u8]) -> Result<Vec<Token>> {
        let (tokens, _) = Self::decode_multiple(param_types, bytes)?;

        Ok(tokens)
    }

    /// The same as `decode` just for a single type. Used in most cases since
    /// contract functions can only return one type.
    pub fn decode_single(param_type: &ParamType, bytes: &[u8]) -> Result<Token> {
        Ok(Self::decode_param(param_type, bytes)?.token)
    }

    fn decode_param(param_type: &ParamType, bytes: &[u8]) -> Result<DecodeResult> {
        match param_type {
            ParamType::Unit => Self::decode_unit(bytes),
            ParamType::U8 => Self::decode_u8(bytes),
            ParamType::U16 => Self::decode_u16(bytes),
            ParamType::U32 => Self::decode_u32(bytes),
            ParamType::U64 => Self::decode_u64(bytes),
            ParamType::Bool => Self::decode_bool(bytes),
            ParamType::Byte => Self::decode_byte(bytes),
            ParamType::B256 => Self::decode_b256(bytes),
            ParamType::String(length) => Self::decode_string(bytes, *length),
            ParamType::Array(ref t, length) => Self::decode_array(t, bytes, *length),
            ParamType::Struct { fields, .. } => Self::decode_struct(fields, bytes),
            ParamType::Enum { variants, .. } => Self::decode_enum(bytes, variants),
            ParamType::Tuple(types) => Self::decode_tuple(types, bytes),
            ParamType::Vector(param_type) => Self::decode_vector(param_type, bytes),
            ParamType::RawSlice => Self::decode_raw_slice(bytes),
        }
    }

    fn decode_vector(_param_type: &ParamType, _bytes: &[u8]) -> Result<DecodeResult> {
        unimplemented!("Cannot decode Vectors until we get support from the compiler.")
    }

    fn decode_tuple(param_types: &[ParamType], bytes: &[u8]) -> Result<DecodeResult> {
        let (tokens, bytes_read) = Self::decode_multiple(param_types, bytes)?;

        Ok(DecodeResult {
            token: Token::Tuple(tokens),
            bytes_read,
        })
    }

    fn decode_struct(param_types: &[(String, ParamType)], bytes: &[u8]) -> Result<DecodeResult> {
        let param_types = unzip_param_types(param_types);
        let (tokens, bytes_read) = Self::decode_multiple(&param_types, bytes)?;

        Ok(DecodeResult {
            token: Token::Struct(tokens),
            bytes_read,
        })
    }

    fn decode_multiple(param_types: &[ParamType], bytes: &[u8]) -> Result<(Vec<Token>, usize)> {
        let mut results = vec![];

        let mut bytes_read = 0;

        for param_type in param_types {
            let res = Self::decode_param(param_type, skip(bytes, bytes_read)?)?;
            bytes_read += res.bytes_read;
            results.push(res.token);
        }

        Ok((results, bytes_read))
    }

    fn decode_array(param_type: &ParamType, bytes: &[u8], length: usize) -> Result<DecodeResult> {
        let (tokens, bytes_read) = Self::decode_multiple(&vec![param_type.clone(); length], bytes)?;

        Ok(DecodeResult {
            token: Token::Array(tokens),
            bytes_read,
        })
    }

    fn decode_raw_slice(bytes: &[u8]) -> Result<DecodeResult> {
        // A raw slice is actually an array of u64.
        let u64_size = std::mem::size_of::<u64>();
        if bytes.len() % u64_size != 0 {
            return Err(error!(
                InvalidData,
                "The bytes provided do not correspond to a raw slice with u64 numbers, got: {:?}",
                bytes
            ));
        }
        let u64_length = bytes.len() / u64_size;
        let (tokens, bytes_read) = Self::decode_multiple(&vec![ParamType::U64; u64_length], bytes)?;
        let elements = tokens
            .into_iter()
            .map(u64::from_token)
            .collect::<Result<Vec<u64>>>()
            .map_err(|e| error!(InvalidData, "{e}"))?;

        Ok(DecodeResult {
            token: Token::RawSlice(elements),
            bytes_read,
        })
    }

    fn decode_string(bytes: &[u8], length: usize) -> Result<DecodeResult> {
        let encoded_len = padded_len_usize(length);
        let encoded_str = peek(bytes, encoded_len)?;

        let decoded = str::from_utf8(&encoded_str[..length])?;

        let result = DecodeResult {
            token: Token::String(StringToken::new(decoded.into(), length)),
            bytes_read: encoded_len,
        };

        Ok(result)
    }

    fn decode_b256(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::B256(*peek_fixed::<32>(bytes)?),
            bytes_read: 32,
        })
    }

    fn decode_byte(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::Byte(peek_u8(bytes)?),
            bytes_read: 8,
        })
    }

    fn decode_bool(bytes: &[u8]) -> Result<DecodeResult> {
        // Grab last byte of the word and compare it to 0x00
        let b = peek_u8(bytes)? != 0u8;

        let result = DecodeResult {
            token: Token::Bool(b),
            bytes_read: 8,
        };

        Ok(result)
    }

    fn decode_u64(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U64(peek_u64(bytes)?),
            bytes_read: 8,
        })
    }

    fn decode_u32(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U32(peek_u32(bytes)?),
            bytes_read: 8,
        })
    }

    fn decode_u16(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U16(peek_u16(bytes)?),
            bytes_read: 8,
        })
    }

    fn decode_u8(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U8(peek_u8(bytes)?),
            bytes_read: 8,
        })
    }

    fn decode_unit(bytes: &[u8]) -> Result<DecodeResult> {
        // We don't need the data, we're doing this purely as a bounds
        // check.
        peek_fixed::<WORD_SIZE>(bytes)?;
        Ok(DecodeResult {
            token: Token::Unit,
            bytes_read: WORD_SIZE,
        })
    }

    /// The encoding follows the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
    ///
    /// # Arguments
    ///
    /// * `data`: slice of encoded data on whose beginning we're expecting an encoded enum
    /// * `variants`: all types that this particular enum type could hold
    fn decode_enum(bytes: &[u8], variants: &EnumVariants) -> Result<DecodeResult> {
        let enum_width = variants.compute_encoding_width_of_enum();

        let discriminant = peek_u32(bytes)? as u8;
        let (_, selected_variant) = variants.select_variant(discriminant)?;

        let words_to_skip = enum_width - selected_variant.compute_encoding_width();
        let enum_content_bytes = skip(bytes, words_to_skip * WORD_SIZE)?;
        let result = Self::decode_token_in_enum(enum_content_bytes, variants, selected_variant)?;

        let selector = Box::new((discriminant, result.token, variants.clone()));
        Ok(DecodeResult {
            token: Token::Enum(selector),
            bytes_read: enum_width * WORD_SIZE,
        })
    }

    fn decode_token_in_enum(
        bytes: &[u8],
        variants: &EnumVariants,
        selected_variant: &ParamType,
    ) -> Result<DecodeResult> {
        // Enums that contain only Units as variants have only their discriminant encoded.
        // Because of this we construct the Token::Unit rather than calling `decode_param`
        if variants.only_units_inside() {
            Ok(DecodeResult {
                token: Token::Unit,
                bytes_read: 0,
            })
        } else {
            Self::decode_param(selected_variant, bytes)
        }
    }
}

fn peek_u64(bytes: &[u8]) -> Result<u64> {
    let slice = peek_fixed::<WORD_SIZE>(bytes)?;
    Ok(u64::from_be_bytes(*slice))
}

fn peek_u32(bytes: &[u8]) -> Result<u32> {
    const BYTES: usize = std::mem::size_of::<u32>();

    let slice = peek_fixed::<WORD_SIZE>(bytes)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u32: You must use a slice containing exactly 4B.");
    Ok(u32::from_be_bytes(bytes))
}

fn peek_u16(bytes: &[u8]) -> Result<u16> {
    const BYTES: usize = std::mem::size_of::<u16>();

    let slice = peek_fixed::<WORD_SIZE>(bytes)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u16: You must use a slice containing exactly 2B.");
    Ok(u16::from_be_bytes(bytes))
}

fn peek_u8(bytes: &[u8]) -> Result<u8> {
    const BYTES: usize = std::mem::size_of::<u8>();

    let slice = peek_fixed::<WORD_SIZE>(bytes)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u8: You must use a slice containing exactly 1B.");
    Ok(u8::from_be_bytes(bytes))
}

fn peek_fixed<const LEN: usize>(data: &[u8]) -> Result<&[u8; LEN]> {
    let slice_w_correct_length = peek(data, LEN)?;
    Ok(<&[u8; LEN]>::try_from(slice_w_correct_length)
        .expect("peek(data,len) must return a slice of length `len` or error out"))
}

fn peek(data: &[u8], len: usize) -> Result<&[u8]> {
    if len > data.len() {
        Err(error!(
            InvalidData,
            "tried to read {len} bytes from response but only had {} remaining!",
            data.len()
        ))
    } else {
        Ok(&data[..len])
    }
}

fn skip(slice: &[u8], num_bytes: usize) -> Result<&[u8]> {
    if num_bytes > slice.len() {
        Err(error!(
            InvalidData,
            "tried to consume {num_bytes} bytes from response but only had {} remaining!",
            slice.len()
        ))
    } else {
        Ok(&slice[num_bytes..])
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use fuels_test_helpers::generate_unused_field_names;
    use fuels_types::{enum_variants::EnumVariants, errors::Error};

    use super::*;

    #[test]
    fn decode_int() -> Result<()> {
        let data = [0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff];

        let decoded = ABIDecoder::decode_single(&ParamType::U32, &data)?;

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
        ];
        let data = [
            0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xff,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff,
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

        let expected = vec![
            Token::U32(u32::MAX),
            Token::U8(u8::MAX),
            Token::U16(u16::MAX),
            Token::U64(u64::MAX),
        ];
        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_bool() -> Result<()> {
        let types = vec![ParamType::Bool, ParamType::Bool];
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x00,
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

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

        let decoded = ABIDecoder::decode_single(&ParamType::B256, &data)?;

        assert_eq!(decoded, Token::B256(data));
        Ok(())
    }

    #[test]
    fn decode_string() -> Result<()> {
        let types = vec![ParamType::String(23), ParamType::String(5)];
        let data = [
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c,
            0x20, 0x73, 0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x00, 0x48, 0x65, 0x6c, 0x6c,
            0x6f, 0x0, 0x0, 0x0,
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

        let expected = vec![
            Token::String(StringToken::new("This is a full sentence".into(), 23)),
            Token::String(StringToken::new("Hello".into(), 5)),
        ];

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_array() -> Result<()> {
        // Create a parameter type for u8[2].
        let types = vec![ParamType::Array(Box::new(ParamType::U8), 2)];
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xff, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2a,
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

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
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
        ];
        let param_type = ParamType::Struct {
            name: "".to_string(),
            fields: generate_unused_field_names(vec![ParamType::U8, ParamType::Bool]),
            generics: vec![],
        };

        let decoded = ABIDecoder::decode_single(&param_type, &data)?;

        let expected = Token::Struct(vec![Token::U8(1), Token::Bool(true)]);

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_enum() -> Result<()> {
        // enum MyEnum {
        //     x: u32,
        //     y: bool,
        // }

        let inner_enum_types = EnumVariants::new(generate_unused_field_names(vec![
            ParamType::U32,
            ParamType::Bool,
        ]))?;
        let types = vec![ParamType::Enum {
            name: "".to_string(),
            variants: inner_enum_types.clone(),
            generics: vec![],
        }];

        // "0" discriminant and 42 enum value
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2a,
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

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

        let inner_enum_types = EnumVariants::new(generate_unused_field_names(vec![
            ParamType::B256,
            ParamType::U32,
        ]))?;

        let struct_type = ParamType::Struct {
            name: "".to_string(),
            fields: generate_unused_field_names(vec![
                ParamType::Enum {
                    name: "".to_string(),
                    variants: inner_enum_types.clone(),
                    generics: vec![],
                },
                ParamType::U32,
            ]),
            generics: vec![],
        };

        let enum_discriminant_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1];
        let enum_data_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x30, 0x39];
        // this padding is due to the biggest variant of MyEnum being 3 WORDs bigger than the chosen variant
        let enum_padding_enc = vec![0x0; 3 * WORD_SIZE];
        let struct_par2_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xD4, 0x31];
        let bytes: Vec<u8> = vec![
            enum_discriminant_enc,
            enum_padding_enc,
            enum_data_enc,
            struct_par2_enc,
        ]
        .into_iter()
        .flatten()
        .collect();

        let decoded = ABIDecoder::decode_single(&struct_type, &bytes)?;

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

        let nested_struct = ParamType::Struct {
            name: "".to_string(),
            fields: generate_unused_field_names(vec![
                ParamType::U16,
                ParamType::Struct {
                    name: "".to_string(),
                    fields: generate_unused_field_names(vec![
                        ParamType::Bool,
                        ParamType::Array(Box::new(ParamType::U8), 2),
                    ]),
                    generics: vec![],
                },
            ]),
            generics: vec![],
        };

        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2,
        ];

        let decoded = ABIDecoder::decode_single(&nested_struct, &data)?;

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

        // fn: long_function(Foo,u8[2],b256,str[23])

        // Parameters
        let nested_struct = ParamType::Struct {
            name: "".to_string(),
            fields: generate_unused_field_names(vec![
                ParamType::U16,
                ParamType::Struct {
                    name: "".to_string(),
                    fields: generate_unused_field_names(vec![
                        ParamType::Bool,
                        ParamType::Array(Box::new(ParamType::U8), 2),
                    ]),
                    generics: vec![],
                },
            ]),
            generics: vec![],
        };

        let u8_arr = ParamType::Array(Box::new(ParamType::U8), 2);
        let b256 = ParamType::B256;
        let s = ParamType::String(23);

        let types = [nested_struct, u8_arr, b256, s];

        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, // foo.x == 10u16
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // foo.y.a == true
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // foo.b.0 == 1u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, // foo.b.1 == 2u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // u8[2].0 == 1u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, // u8[2].0 == 2u8
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, // b256
            0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44, 0xe4, 0xcb, // b256
            0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, // b256
            0xa8, 0xf8, 0x27, 0x43, 0xf3, 0x1e, 0x93, 0xb, // b256
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // str[23]
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // str[23]
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x0, // str[23]
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

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

        let s = Token::String(StringToken::new("This is a full sentence".into(), 23));

        let expected: Vec<Token> = vec![foo, u8_arr, b256, s];

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn units_in_structs_are_decoded_as_one_word() -> Result<()> {
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let struct_type = ParamType::Struct {
            name: "".to_string(),
            fields: generate_unused_field_names(vec![ParamType::Unit, ParamType::U64]),
            generics: vec![],
        };

        let actual = ABIDecoder::decode_single(&struct_type, &data)?;

        let expected = Token::Struct(vec![Token::Unit, Token::U64(u64::MAX)]);
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn enums_with_all_unit_variants_are_decoded_from_one_word() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1];
        let variants = EnumVariants::new(generate_unused_field_names(vec![
            ParamType::Unit,
            ParamType::Unit,
        ]))?;
        let enum_w_only_units = ParamType::Enum {
            name: "".to_string(),
            variants: variants.clone(),
            generics: vec![],
        };

        let result = ABIDecoder::decode_single(&enum_w_only_units, &data)?;

        let expected_enum = Token::Enum(Box::new((1, Token::Unit, variants)));
        assert_eq!(result, expected_enum);
        Ok(())
    }

    #[test]
    fn out_of_bounds_discriminant_is_detected() -> Result<()> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let variants = EnumVariants::new(generate_unused_field_names(vec![ParamType::U32]))?;
        let enum_type = ParamType::Enum {
            name: "".to_string(),
            variants,
            generics: vec![],
        };

        let result = ABIDecoder::decode_single(&enum_type, &data);

        let error = result.expect_err("Should have resulted in an error");

        let expected_msg = "Discriminant '1' doesn't point to any variant: ";
        assert!(matches!(error, Error::InvalidData(str) if str.starts_with(expected_msg)));
        Ok(())
    }
}
