use std::{convert::TryInto, str};

use crate::{
    checked_round_up_to_word_alignment,
    codec::{
        utils::{CodecDirection, CounterWithLimit},
        DecoderConfig,
    },
    constants::WORD_SIZE,
    types::{
        errors::{error, Result},
        param_types::{EnumVariants, NamedParamType, ParamType},
        StaticStringToken, Token, U256,
    },
};

/// Is used to decode bytes into `Token`s from which types implementing `Tokenizable` can be
/// instantiated. Implements decoding limits to control resource usage.
pub(crate) struct BoundedDecoder {
    depth_tracker: CounterWithLimit,
    token_tracker: CounterWithLimit,
    config: DecoderConfig,
}

const U128_BYTES_SIZE: usize = 2 * WORD_SIZE;
const U256_BYTES_SIZE: usize = 4 * WORD_SIZE;
const B256_BYTES_SIZE: usize = 4 * WORD_SIZE;

impl BoundedDecoder {
    pub(crate) fn new(config: DecoderConfig) -> Self {
        let depth_tracker =
            CounterWithLimit::new(config.max_depth, "depth", CodecDirection::Decoding);
        let token_tracker =
            CounterWithLimit::new(config.max_tokens, "token", CodecDirection::Decoding);
        Self {
            depth_tracker,
            token_tracker,
            config,
        }
    }

    pub(crate) fn decode(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Token> {
        param_type.validate_is_decodable(self.config.max_depth)?;
        match param_type {
            // Unit, U8 and Bool are returned as u64 from receipt "Return"
            ParamType::Unit => Ok(Token::Unit),
            ParamType::U8 => Self::decode_u64(bytes).map(|r| {
                Token::U8(match r.token {
                    Token::U64(v) => v as u8,
                    _ => unreachable!("decode_u64 returning unexpected token"),
                })
            }),
            ParamType::Bool => Self::decode_u64(bytes).map(|r| {
                Token::Bool(match r.token {
                    Token::U64(v) => v != 0,
                    _ => unreachable!("decode_u64 returning unexpected token"),
                })
            }),
            _ => self.decode_param(param_type, bytes).map(|x| x.token),
        }
    }

    pub(crate) fn decode_multiple(
        &mut self,
        param_types: &[ParamType],
        bytes: &[u8],
    ) -> Result<Vec<Token>> {
        for param_type in param_types {
            param_type.validate_is_decodable(self.config.max_depth)?;
        }
        let (tokens, _) = self.decode_params(param_types, bytes)?;

        Ok(tokens)
    }

    fn run_w_depth_tracking(
        &mut self,
        decoder: impl FnOnce(&mut Self) -> Result<Decoded>,
    ) -> Result<Decoded> {
        self.depth_tracker.increase()?;

        let res = decoder(self);

        self.depth_tracker.decrease();
        res
    }

    fn decode_param(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Decoded> {
        self.token_tracker.increase()?;
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
            ParamType::StringArray(len) => Self::decode_string_array(bytes, *len),
            ParamType::Array(ref t, length) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_array(t, bytes, *length))
            }
            ParamType::Struct { fields, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_struct(fields, bytes))
            }
            ParamType::Enum { enum_variants, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_enum(bytes, enum_variants))
            }
            ParamType::Tuple(types) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_tuple(types, bytes))
            }
            ParamType::Vector(param_type) => {
                // although nested vectors cannot be decoded yet, depth tracking still occurs for future
                // proofing
                self.run_w_depth_tracking(|ctx| ctx.decode_vector(param_type, bytes))
            }
            ParamType::Bytes => Self::decode_bytes(bytes),
            ParamType::String => Self::decode_std_string(bytes),
        }
    }

    fn decode_bytes(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::Bytes(bytes.to_vec()),
            bytes_read: bytes.len(),
        })
    }

    fn decode_std_string(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::String(str::from_utf8(bytes)?.to_string()),
            bytes_read: bytes.len(),
        })
    }

    fn decode_vector(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Decoded> {
        let num_of_elements = ParamType::calculate_num_of_elements(param_type, bytes.len())?;
        let (tokens, bytes_read) =
            self.decode_params(std::iter::repeat(param_type).take(num_of_elements), bytes)?;

        Ok(Decoded {
            token: Token::Vector(tokens),
            bytes_read,
        })
    }

    fn decode_tuple(&mut self, param_types: &[ParamType], bytes: &[u8]) -> Result<Decoded> {
        let mut tokens = vec![];

        let mut bytes_read = 0;

        for param_type in param_types.iter() {
            // padding has to be taken into account
            bytes_read = checked_round_up_to_word_alignment(bytes_read)?;
            let res = self.decode_param(param_type, skip(bytes, bytes_read)?)?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        Ok(Decoded {
            token: Token::Tuple(tokens),
            bytes_read,
        })
    }

    fn decode_struct(&mut self, param_types: &[NamedParamType], bytes: &[u8]) -> Result<Decoded> {
        let mut tokens = vec![];

        let mut bytes_read = 0;

        for (_, param_type) in param_types.iter() {
            // padding has to be taken into account
            bytes_read = checked_round_up_to_word_alignment(bytes_read)?;
            let res = self.decode_param(param_type, skip(bytes, bytes_read)?)?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        Ok(Decoded {
            token: Token::Struct(tokens),
            bytes_read,
        })
    }

    fn decode_params<'a>(
        &mut self,
        param_types: impl IntoIterator<Item = &'a ParamType>,
        bytes: &[u8],
    ) -> Result<(Vec<Token>, usize)> {
        let mut results = vec![];

        let mut bytes_read = 0;

        for param_type in param_types {
            let res = self.decode_param(param_type, skip(bytes, bytes_read)?)?;
            bytes_read += res.bytes_read;
            results.push(res.token);
        }

        Ok((results, bytes_read))
    }

    fn decode_array(
        &mut self,
        param_type: &ParamType,
        bytes: &[u8],
        length: usize,
    ) -> Result<Decoded> {
        let (tokens, bytes_read) =
            self.decode_params(std::iter::repeat(param_type).take(length), bytes)?;

        Ok(Decoded {
            token: Token::Array(tokens),
            bytes_read,
        })
    }

    fn decode_raw_slice(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::RawSlice(bytes.to_vec()),
            bytes_read: bytes.len(),
        })
    }

    fn decode_string_slice(bytes: &[u8]) -> Result<Decoded> {
        let decoded = str::from_utf8(bytes)?;

        Ok(Decoded {
            token: Token::StringSlice(StaticStringToken::new(decoded.into(), None)),
            bytes_read: decoded.len(),
        })
    }

    fn decode_string_array(bytes: &[u8], length: usize) -> Result<Decoded> {
        let encoded_str = peek(bytes, length)?;

        let decoded = str::from_utf8(encoded_str)?;
        let result = Decoded {
            token: Token::StringArray(StaticStringToken::new(decoded.into(), Some(length))),
            bytes_read: checked_round_up_to_word_alignment(length)?,
        };
        Ok(result)
    }

    fn decode_b256(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::B256(*peek_fixed::<32>(bytes)?),
            bytes_read: B256_BYTES_SIZE,
        })
    }

    fn decode_bool(bytes: &[u8]) -> Result<Decoded> {
        // Grab last byte of the word and compare it to 0x00
        let b = peek_u8(bytes)? != 0u8;

        let result = Decoded {
            token: Token::Bool(b),
            bytes_read: 1,
        };

        Ok(result)
    }

    fn decode_u128(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U128(peek_u128(bytes)?),
            bytes_read: U128_BYTES_SIZE,
        })
    }

    fn decode_u256(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U256(peek_u256(bytes)?),
            bytes_read: U256_BYTES_SIZE,
        })
    }

    fn decode_u64(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U64(peek_u64(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u32(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U32(peek_u32(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u16(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U16(peek_u16(bytes)?),
            bytes_read: WORD_SIZE,
        })
    }

    fn decode_u8(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U8(peek_u8(bytes)?),
            bytes_read: 1,
        })
    }

    fn decode_unit(bytes: &[u8]) -> Result<Decoded> {
        // We don't need the data, we're doing this purely as a bounds
        // check.
        peek_fixed::<1>(bytes)?;
        Ok(Decoded {
            token: Token::Unit,
            bytes_read: 1,
        })
    }

    /// The encoding follows the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
    ///
    /// # Arguments
    ///
    /// * `data`: slice of encoded data on whose beginning we're expecting an encoded enum
    /// * `variants`: all types that this particular enum type could hold
    fn decode_enum(&mut self, bytes: &[u8], enum_variants: &EnumVariants) -> Result<Decoded> {
        let enum_width_in_bytes = enum_variants.compute_enum_width_in_bytes()?;

        let discriminant = peek_u64(bytes)?;
        let (_, selected_variant) = enum_variants.select_variant(discriminant)?;

        let skip_extra_in_bytes = match enum_variants.heap_type_variant() {
            Some((heap_type_discriminant, heap_type)) if heap_type_discriminant == discriminant => {
                heap_type.compute_encoding_in_bytes()?
            }
            _ => 0,
        };

        let bytes_to_skip = enum_width_in_bytes - selected_variant.compute_encoding_in_bytes()?
            + skip_extra_in_bytes;

        let enum_content_bytes = skip(bytes, bytes_to_skip)?;
        let result =
            self.decode_token_in_enum(enum_content_bytes, enum_variants, selected_variant)?;

        let selector = Box::new((discriminant, result.token, enum_variants.clone()));
        Ok(Decoded {
            token: Token::Enum(selector),
            bytes_read: enum_width_in_bytes,
        })
    }

    fn decode_token_in_enum(
        &mut self,
        bytes: &[u8],
        variants: &EnumVariants,
        selected_variant: &ParamType,
    ) -> Result<Decoded> {
        // Enums that contain only Units as variants have only their discriminant encoded.
        // Because of this we construct the Token::Unit rather than calling `decode_param`
        if variants.only_units_inside() {
            Ok(Decoded {
                token: Token::Unit,
                bytes_read: 0,
            })
        } else {
            self.decode_param(selected_variant, bytes)
        }
    }
}

#[derive(Debug, Clone)]
struct Decoded {
    token: Token,
    bytes_read: usize,
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
        .expect("peek_u32: You must use a slice containing exactly 4B");
    Ok(u32::from_be_bytes(bytes))
}

fn peek_u16(bytes: &[u8]) -> Result<u16> {
    const BYTES: usize = std::mem::size_of::<u16>();

    let slice = peek_fixed::<WORD_SIZE>(bytes)?;
    let bytes = slice[WORD_SIZE - BYTES..]
        .try_into()
        .expect("peek_u16: You must use a slice containing exactly 2B");
    Ok(u16::from_be_bytes(bytes))
}

fn peek_u8(bytes: &[u8]) -> Result<u8> {
    const BYTES: usize = std::mem::size_of::<u8>();

    let slice = peek_fixed::<1>(bytes)?;
    let bytes = slice[1 - BYTES..]
        .try_into()
        .expect("peek_u8: You must use a slice containing exactly 1B");
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
            Codec,
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
            Codec,
            "tried to consume {num_bytes} bytes from response but only had {} remaining!",
            slice.len()
        ))
    } else {
        Ok(&slice[num_bytes..])
    }
}
