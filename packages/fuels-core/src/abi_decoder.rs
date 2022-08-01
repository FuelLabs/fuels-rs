use crate::{StringToken, Token};
use core::convert::TryInto;
use core::str;
use fuel_types::bytes::padded_len;
use fuels_types::{
    constants::WORD_SIZE,
    errors::CodecError,
    param_types::{EnumVariants, ParamType},
};

#[derive(Debug, Clone)]
struct DecodeResult {
    token: Token,
    bytes_read: usize,
}

pub struct ABIDecoder;

impl ABIDecoder {
    /// Decode takes an array of `ParamType` and the encoded data as raw bytes
    /// and returns a vector of `Token`s containing the decoded values.
    /// Note that the order of the types in the `types` array needs to match the order
    /// of the expected values/types in `data`.
    /// You can find comprehensive examples in the tests for this module.
    pub fn decode(types: &[ParamType], data: &[u8]) -> Result<Vec<Token>, CodecError> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut offset = 0;
        for param in types {
            let res = Self::decode_param(param, &data[offset..])?;
            offset += res.bytes_read;
            tokens.push(res.token);
        }

        Ok(tokens)
    }

    pub fn decode_single(param: &ParamType, data: &[u8]) -> Result<Token, CodecError> {
        Ok(Self::decode_param(param, data)?.token)
    }

    fn decode_param(param: &ParamType, data: &[u8]) -> Result<DecodeResult, CodecError> {
        match param {
            ParamType::Unit => Self::decode_unit(data),
            ParamType::U8 => Self::decode_u8(data),
            ParamType::U16 => Self::decode_u16(data),
            ParamType::U32 => Self::decode_u32(data),
            ParamType::U64 => Self::decode_u64(data),
            ParamType::Bool => Self::decode_bool(data),
            ParamType::Byte => Self::decode_byte(data),
            ParamType::B256 => Self::decode_b256(data),
            ParamType::String(length) => Self::decode_string(data, *length),
            ParamType::Array(ref t, length) => Self::decode_array(data, t, *length),
            ParamType::Struct(props) => Self::decode_struct(data, props),
            ParamType::Enum(variants) => Self::decode_enum(data, variants),
            ParamType::Tuple(types) => Self::decode_tuple(data, types),
        }
    }

    fn decode_tuple(data: &[u8], types: &Vec<ParamType>) -> Result<DecodeResult, CodecError> {
        let mut tokens = vec![];
        let mut bytes_read = 0;
        for t in types {
            let res = Self::decode_param(t, &data[bytes_read..])?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        let result = DecodeResult {
            token: Token::Tuple(tokens),
            bytes_read,
        };

        Ok(result)
    }

    fn decode_struct(data: &[u8], props: &Vec<ParamType>) -> Result<DecodeResult, CodecError> {
        let mut tokens = vec![];

        let mut bytes_read = 0;
        for prop in props {
            let res = Self::decode_param(prop, &data[bytes_read..])?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        let result = DecodeResult {
            token: Token::Struct(tokens),
            bytes_read,
        };

        Ok(result)
    }

    fn decode_array(data: &[u8], t: &ParamType, length: usize) -> Result<DecodeResult, CodecError> {
        let mut tokens = vec![];
        let mut bytes_read = 0;

        for _ in 0..length {
            let res = Self::decode_param(t, &data[bytes_read..])?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        let result = DecodeResult {
            token: Token::Array(tokens),
            bytes_read,
        };

        Ok(result)
    }

    fn decode_string(data: &[u8], length: usize) -> Result<DecodeResult, CodecError> {
        let encoded_str = peek(data, length)?;

        let decoded = str::from_utf8(encoded_str)?;

        let result = DecodeResult {
            token: Token::String(StringToken::new(decoded.into(), length)),
            bytes_read: padded_len(encoded_str),
        };

        Ok(result)
    }

    fn decode_b256(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::B256(*peek_fixed::<32>(data)?),
            bytes_read: 32,
        })
    }

    fn decode_byte(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::Byte(peek_u8(data)?),
            bytes_read: 8,
        })
    }

    fn decode_bool(data: &[u8]) -> Result<DecodeResult, CodecError> {
        // Grab last byte of the word and compare it to 0x00
        let b = peek_u8(data)? != 0u8;

        let result = DecodeResult {
            token: Token::Bool(b),
            bytes_read: 8,
        };

        Ok(result)
    }

    fn decode_u64(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::U64(peek_u64(data)?),
            bytes_read: 8,
        })
    }

    fn decode_u32(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::U32(peek_u32(data)?),
            bytes_read: 8,
        })
    }

    fn decode_u16(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::U16(peek_u16(data)?),
            bytes_read: 8,
        })
    }

    fn decode_u8(data: &[u8]) -> Result<DecodeResult, CodecError> {
        Ok(DecodeResult {
            token: Token::U8(peek_u8(data)?),
            bytes_read: 8,
        })
    }

    fn decode_unit(data: &[u8]) -> Result<DecodeResult, CodecError> {
        // We don't need the data, we're doing this purely as a bounds
        // check.
        peek_fixed::<WORD_SIZE>(data)?;
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
    fn decode_enum(data: &[u8], variants: &EnumVariants) -> Result<DecodeResult, CodecError> {
        let discriminant = peek_u32(data)?;
        let selected_variant = Self::type_of_selected_variant(variants, discriminant as usize)?;

        let enum_width = variants.compute_encoding_width_of_enum();
        let token = Self::decode_token_in_enum(data, variants, selected_variant, enum_width)?;

        let selector = Box::new((discriminant as u8, token, variants.clone()));
        Ok(DecodeResult {
            token: Token::Enum(selector),
            bytes_read: enum_width * WORD_SIZE,
        })
    }

    fn decode_token_in_enum(
        data: &[u8],
        variants: &EnumVariants,
        selected_variant: &ParamType,
        enum_width: usize,
    ) -> Result<Token, CodecError> {
        // The sway compiler has an optimization where enums that only contain
        // units for variants have only their discriminant encoded. Because of
        // this we construct the Token::Unit rather than calling `decode_param`
        // since that will consume a WORD from `data`.
        if variants.only_units_inside() {
            Ok(Token::Unit)
        } else {
            let words_to_skip = enum_width - selected_variant.compute_encoding_width();

            let res = Self::decode_param(selected_variant, &data[words_to_skip * WORD_SIZE..])?;
            Ok(res.token)
        }
    }

    /// Returns a variant from `variants` pointed to by `discriminant`.
    /// Will fail if `discriminant` is out of bounds.
    fn type_of_selected_variant(
        variants: &EnumVariants,
        discriminant: usize,
    ) -> Result<&ParamType, CodecError> {
        variants.param_types().get(discriminant).ok_or_else(|| {
            let msg = format!(
                concat!(
                    "Error while decoding an enum. The discriminant '{}' doesn't ",
                    "point to any of the following variants: {:?}"
                ),
                discriminant, variants
            );
            CodecError::InvalidData(msg)
        })
    }
}

fn peek_u64(data: &[u8]) -> Result<u64, CodecError> {
    let slice = peek_fixed::<WORD_SIZE>(data)?;
    Ok(u64::from_be_bytes(*slice))
}

fn peek_u32(data: &[u8]) -> Result<u32, CodecError> {
    const BYTES: usize = std::mem::size_of::<u32>();

    let slice = peek_fixed::<WORD_SIZE>(data)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u32: You must use a slice containing exactly 4B.");
    Ok(u32::from_be_bytes(bytes))
}

fn peek_u16(data: &[u8]) -> Result<u16, CodecError> {
    const BYTES: usize = std::mem::size_of::<u16>();

    let slice = peek_fixed::<WORD_SIZE>(data)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u16: You must use a slice containing exactly 2B.");
    Ok(u16::from_be_bytes(bytes))
}

fn peek_u8(data: &[u8]) -> Result<u8, CodecError> {
    const BYTES: usize = std::mem::size_of::<u8>();

    let slice = peek_fixed::<WORD_SIZE>(data)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u8: You must use a slice containing exactly 1B.");
    Ok(u8::from_be_bytes(bytes))
}

fn peek_fixed<const LEN: usize>(data: &[u8]) -> Result<&[u8; LEN], CodecError> {
    let slice_w_correct_length = peek(data, LEN)?;
    Ok(<&[u8; LEN]>::try_from(slice_w_correct_length)
        .expect("peek(data,len) must return a slice of length `len` or error out"))
}

fn peek(data: &[u8], len: usize) -> Result<&[u8], CodecError> {
    if len > data.len() {
        Err(CodecError::InvalidData(
            "requested data out of bounds".into(),
        ))
    } else {
        Ok(&data[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::{errors::Error, param_types::EnumVariants};

    #[test]
    fn decode_int() -> Result<(), Error> {
        let data = [0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff];

        let decoded = ABIDecoder::decode_single(&ParamType::U32, &data)?;

        assert_eq!(decoded, Token::U32(u32::MAX));
        Ok(())
    }

    #[test]
    fn decode_multiple_int() -> Result<(), Error> {
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
    fn decode_bool() -> Result<(), Error> {
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
    fn decode_b256() -> Result<(), Error> {
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
    fn decode_string() -> Result<(), Error> {
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
    fn decode_array() -> Result<(), Error> {
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
    fn decode_struct() -> Result<(), Error> {
        // Sway struct:
        // struct MyStruct {
        //     foo: u8,
        //     bar: bool,
        // }

        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
        ];
        let param_type = ParamType::Struct(vec![ParamType::U8, ParamType::Bool]);

        let decoded = ABIDecoder::decode_single(&param_type, &data)?;

        let expected = Token::Struct(vec![Token::U8(1), Token::Bool(true)]);

        assert_eq!(decoded, expected);
        Ok(())
    }

    #[test]
    fn decode_enum() -> Result<(), Error> {
        // Sway enum:
        // enum MyEnum {
        //     x: u32,
        //     y: bool,
        // }

        let inner_enum_types = EnumVariants::new(vec![ParamType::U32, ParamType::Bool])?;
        let types = vec![ParamType::Enum(inner_enum_types.clone())];

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
    fn decoder_will_skip_enum_padding_and_decode_next_arg() -> Result<(), Error> {
        // struct MyStruct {
        //     par1: MyEnum,
        //     par2: u32
        // }

        // enum MyEnum {
        //     x: b256,
        //     y: u32,
        // }

        let inner_enum_types = EnumVariants::new(vec![ParamType::B256, ParamType::U32])?;

        let struct_type = ParamType::Struct(vec![
            ParamType::Enum(inner_enum_types.clone()),
            ParamType::U32,
        ]);

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
    fn decode_nested_struct() -> Result<(), Error> {
        // Sway nested struct:
        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        let nested_struct = ParamType::Struct(vec![
            ParamType::U16,
            ParamType::Struct(vec![
                ParamType::Bool,
                ParamType::Array(Box::new(ParamType::U8), 2),
            ]),
        ]);

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
    fn decode_comprehensive() -> Result<(), Error> {
        // Sway nested struct:
        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        // Sway fn: long_function(Foo,u8[2],b256,str[23])

        // Parameters
        let nested_struct = ParamType::Struct(vec![
            ParamType::U16,
            ParamType::Struct(vec![
                ParamType::Bool,
                ParamType::Array(Box::new(ParamType::U8), 2),
            ]),
        ]);

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
    fn units_in_structs_are_decoded_as_one_word() -> Result<(), Error> {
        let data = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let struct_type = ParamType::Struct(vec![ParamType::Unit, ParamType::U64]);

        let actual = ABIDecoder::decode_single(&struct_type, &data)?;

        let expected = Token::Struct(vec![Token::Unit, Token::U64(u64::MAX)]);
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn enums_with_all_unit_variants_are_decoded_from_one_word() -> Result<(), Error> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1];
        let variants = EnumVariants::new(vec![ParamType::Unit, ParamType::Unit])?;
        let enum_w_only_units = ParamType::Enum(variants.clone());

        let result = ABIDecoder::decode_single(&enum_w_only_units, &data)?;

        let expected_enum = Token::Enum(Box::new((1, Token::Unit, variants)));
        assert_eq!(result, expected_enum);
        Ok(())
    }

    #[test]
    fn out_of_bounds_discriminant_is_detected() -> Result<(), Error> {
        let data = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let variants = EnumVariants::new(vec![ParamType::U32])?;
        let enum_type = ParamType::Enum(variants);

        let result = ABIDecoder::decode_single(&enum_type, &data);

        let error = result.expect_err("Should have resulted in an error");

        let expected_msg = "Error while decoding an enum. The discriminant '1' doesn't point to any of the following variants: ";
        assert!(matches!(error, CodecError::InvalidData(str) if str.starts_with(expected_msg)));
        Ok(())
    }
}
