use std::{convert::TryInto, str};

use fuel_types::bytes::padded_len_usize;

use crate::{
    constants::WORD_SIZE,
    traits::Tokenizable,
    types::{
        enum_variants::EnumVariants,
        errors::{error, Error, Result},
        param_types::ParamType,
        StringToken, Token, U256,
    },
};

const U128_BYTES_SIZE: usize = 2 * WORD_SIZE;
const U256_BYTES_SIZE: usize = 4 * WORD_SIZE;
const B256_BYTES_SIZE: usize = 4 * WORD_SIZE;

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
    /// use fuels_core::codec::ABIDecoder;
    /// use fuels_core::types::{param_types::ParamType, Token};
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
        if param_type.contains_nested_heap_types() {
            return Err(error!(
                InvalidData,
                "Type {param_type:?} contains nested heap types (`Vec` or `Bytes`), this is not supported."
            ));
        }
        match param_type {
            ParamType::Unit => Self::decode_unit(bytes),
            ParamType::U8 => Self::decode_u8(bytes),
            ParamType::U16 => Self::decode_u16(bytes),
            ParamType::U32 => Self::decode_u32(bytes),
            ParamType::U64 => Self::decode_u64(bytes),
            ParamType::U128 => Self::decode_u128(bytes),
            ParamType::U256 => Self::decode_u256(bytes),
            ParamType::Bool => Self::decode_bool(bytes),
            ParamType::B256 => Self::decode_b256(bytes),
            ParamType::RawSlice => Self::decode_raw_slice(bytes),
            ParamType::StringSlice => Self::decode_string_slice(bytes),
            ParamType::String(len) => Self::decode_string_array(bytes, *len),
            ParamType::Array(ref t, length) => Self::decode_array(t, bytes, *length),
            ParamType::Struct { fields, .. } => Self::decode_struct(fields, bytes),
            ParamType::Enum { variants, .. } => Self::decode_enum(bytes, variants),
            ParamType::Tuple(types) => Self::decode_tuple(types, bytes),
            ParamType::Vector(param_type) => Self::decode_vector(param_type, bytes),
            ParamType::Bytes => Self::decode_bytes(bytes),
            ParamType::StdString => Self::decode_std_string(bytes),
        }
    }

    fn decode_bytes(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::Bytes(bytes.to_vec()),
            bytes_read: bytes.len(),
        })
    }

    fn decode_std_string(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::StdString(str::from_utf8(bytes)?.to_string()),
            bytes_read: bytes.len(),
        })
    }

    fn decode_vector(param_type: &ParamType, bytes: &[u8]) -> Result<DecodeResult> {
        let num_of_elements = ParamType::calculate_num_of_elements(param_type, bytes.len())?;
        let (tokens, bytes_read) = Self::decode_multiple(vec![param_type; num_of_elements], bytes)?;

        Ok(DecodeResult {
            token: Token::Vector(tokens),
            bytes_read,
        })
    }

    fn decode_tuple(param_types: &[ParamType], bytes: &[u8]) -> Result<DecodeResult> {
        let (tokens, bytes_read) = Self::decode_multiple(param_types, bytes)?;

        Ok(DecodeResult {
            token: Token::Tuple(tokens),
            bytes_read,
        })
    }

    fn decode_struct(param_types: &[ParamType], bytes: &[u8]) -> Result<DecodeResult> {
        let (tokens, bytes_read) = Self::decode_multiple(param_types, bytes)?;

        Ok(DecodeResult {
            token: Token::Struct(tokens),
            bytes_read,
        })
    }

    fn decode_multiple<'a>(
        param_types: impl IntoIterator<Item = &'a ParamType>,
        bytes: &[u8],
    ) -> Result<(Vec<Token>, usize)> {
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
        let raw_slice_element = ParamType::U64;
        let num_of_elements =
            ParamType::calculate_num_of_elements(&raw_slice_element, bytes.len())?;
        let (tokens, bytes_read) =
            Self::decode_multiple(&vec![ParamType::U64; num_of_elements], bytes)?;
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

    fn decode_string_slice(bytes: &[u8]) -> Result<DecodeResult> {
        let decoded = str::from_utf8(bytes)?;

        Ok(DecodeResult {
            token: Token::StringSlice(StringToken::new(decoded.into(), None)),
            bytes_read: decoded.len(),
        })
    }

    fn decode_string_array(bytes: &[u8], length: usize) -> Result<DecodeResult> {
        let encoded_len = padded_len_usize(length);
        let encoded_str = peek(bytes, encoded_len)?;

        let decoded = str::from_utf8(&encoded_str[..length])?;
        let result = DecodeResult {
            token: Token::StringArray(StringToken::new(decoded.into(), Some(length))),
            bytes_read: encoded_len,
        };
        Ok(result)
    }

    fn decode_b256(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::B256(*peek_fixed::<32>(bytes)?),
            bytes_read: B256_BYTES_SIZE,
        })
    }

    fn decode_bool(bytes: &[u8]) -> Result<DecodeResult> {
        // Grab last byte of the word and compare it to 0x00
        let b = peek_u8(bytes)? != 0u8;

        let result = DecodeResult {
            token: Token::Bool(b),
            bytes_read: WORD_SIZE,
        };

        Ok(result)
    }

    fn decode_u128(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U128(peek_u128(bytes)?),
            bytes_read: U128_BYTES_SIZE,
        })
    }

    fn decode_u256(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U256(peek_u256(bytes)?),
            bytes_read: U256_BYTES_SIZE,
        })
    }

    fn decode_u64(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U64(peek_u64(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u32(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U32(peek_u32(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u16(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U16(peek_u16(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u8(bytes: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult {
            token: Token::U8(peek_u8(bytes)?),
            bytes_read: WORD_SIZE,
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
        let selected_variant = variants.param_type_of_variant(discriminant)?;

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

fn peek_u128(bytes: &[u8]) -> Result<u128> {
    let slice = peek_fixed::<U128_BYTES_SIZE>(bytes)?;
    Ok(u128::from_be_bytes(*slice))
}

fn peek_u256(bytes: &[u8]) -> Result<U256> {
    let slice = peek_fixed::<U256_BYTES_SIZE>(bytes)?;
    Ok(U256::from(*slice))
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
    fn decode_string_array() -> Result<()> {
        let types = vec![ParamType::String(23), ParamType::String(5)];
        let data = [
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // This is
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // a full s
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x00, // entence
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x00, 0x00, // Hello
        ];

        let decoded = ABIDecoder::decode(&types, &data)?;

        let expected = vec![
            Token::StringArray(StringToken::new("This is a full sentence".into(), Some(23))),
            Token::StringArray(StringToken::new("Hello".into(), Some(5))),
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

        let decoded = ABIDecoder::decode(&types, &data)?;

        let expected = vec![Token::StringSlice(StringToken::new(
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
            fields: vec![ParamType::U8, ParamType::Bool],
            generics: vec![],
        };

        let decoded = ABIDecoder::decode_single(&param_type, &data)?;

        let expected = Token::Struct(vec![Token::U8(1), Token::Bool(true)]);

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_bytes() -> Result<()> {
        let data = [0xFF, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let decoded = ABIDecoder::decode_single(&ParamType::Bytes, &data)?;

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

        let decoded = ABIDecoder::decode_single(&struct_type, &data)?;

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
        let s = ParamType::String(3);
        let ss = ParamType::StringSlice;

        let types = [nested_struct, u8_arr, b256, s, ss];

        let bytes = [
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
            0x66, 0x6f, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, // str[3]
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // str data
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // str data
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, // str data
        ];

        let decoded = ABIDecoder::decode(&types, &bytes)?;

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

        let ss = Token::StringSlice(StringToken::new("This is a full sentence".into(), None));

        let s = Token::StringArray(StringToken::new("foo".into(), Some(3)));

        let expected: Vec<Token> = vec![foo, u8_arr, b256, s, ss];

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

        let actual = ABIDecoder::decode_single(&struct_type, &data)?;

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

        let result = ABIDecoder::decode_single(&enum_w_only_units, &data)?;

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

        let result = ABIDecoder::decode_single(&enum_type, &data);

        let error = result.expect_err("Should have resulted in an error");

        let expected_msg = "Discriminant '1' doesn't point to any variant: ";
        assert!(matches!(error, Error::InvalidData(str) if str.starts_with(expected_msg)));
        Ok(())
    }
}
