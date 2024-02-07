use std::{iter::repeat, str};

use crate::{
    codec::DecoderConfig,
    constants::WORD_SIZE,
    types::{
        enum_variants::EnumVariants,
        errors::{error, Result},
        param_types::ParamType,
        StaticStringToken, Token, U256,
    },
};

/// Is used to decode bytes into `Token`s from which types implementing `Tokenizable` can be
/// instantiated. Implements decoding limits to control resource usage.
pub(crate) struct ExperimentalBoundedDecoder {
    depth_tracker: CounterWithLimit,
    token_tracker: CounterWithLimit,
}

const U8_BYTES_SIZE: usize = 1;
const U16_BYTES_SIZE: usize = 2;
const U32_BYTES_SIZE: usize = 4;
const U64_BYTES_SIZE: usize = WORD_SIZE;
const U128_BYTES_SIZE: usize = 2 * WORD_SIZE;
const U256_BYTES_SIZE: usize = 4 * WORD_SIZE;
const B256_BYTES_SIZE: usize = 4 * WORD_SIZE;
const LENGTH_BYTES_SIZE: usize = WORD_SIZE;
const DISCRIMINANT_BYTES_SIZE: usize = WORD_SIZE;

impl ExperimentalBoundedDecoder {
    pub(crate) fn new(config: DecoderConfig) -> Self {
        let depth_tracker = CounterWithLimit::new(config.max_depth, "depth");
        let token_tracker = CounterWithLimit::new(config.max_tokens, "token");
        Self {
            depth_tracker,
            token_tracker,
        }
    }

    pub(crate) fn decode(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Token> {
        self.decode_param(param_type, bytes).map(|x| x.token)
    }

    pub(crate) fn decode_multiple(
        &mut self,
        param_types: &[ParamType],
        bytes: &[u8],
    ) -> Result<Vec<Token>> {
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
            ParamType::Unit => Self::decode_unit(),
            ParamType::Bool => Self::decode_bool(bytes),
            ParamType::U8 => Self::decode_u8(bytes),
            ParamType::U16 => Self::decode_u16(bytes),
            ParamType::U32 => Self::decode_u32(bytes),
            ParamType::U64 => Self::decode_u64(bytes),
            ParamType::U128 => Self::decode_u128(bytes),
            ParamType::U256 => Self::decode_u256(bytes),
            ParamType::B256 => Self::decode_b256(bytes),
            ParamType::Bytes => Self::decode_bytes(bytes),
            ParamType::String => Self::decode_std_string(bytes),
            ParamType::RawSlice => Self::decode_raw_slice(bytes),
            ParamType::StringArray(length) => Self::decode_string_array(bytes, *length),
            ParamType::StringSlice => Self::decode_string_slice(bytes),
            ParamType::Tuple(param_types) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_tuple(param_types, bytes))
            }
            ParamType::Array(param_type, length) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_array(param_type, bytes, *length))
            }
            ParamType::Vector(param_type) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_vector(param_type, bytes))
            }

            ParamType::Struct { fields, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_struct(fields, bytes))
            }
            ParamType::Enum { variants, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_enum(bytes, variants))
            }
        }
    }

    fn decode_unit() -> Result<Decoded> {
        Ok(Decoded {
            token: Token::Unit,
            bytes_read: 0,
        })
    }

    fn decode_bool(bytes: &[u8]) -> Result<Decoded> {
        let value = peek_u8(bytes)? != 0u8;

        Ok(Decoded {
            token: Token::Bool(value),
            bytes_read: U8_BYTES_SIZE,
        })
    }

    fn decode_u8(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U8(peek_u8(bytes)?),
            bytes_read: U8_BYTES_SIZE,
        })
    }

    fn decode_u16(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U16(peek_u16(bytes)?),
            bytes_read: U16_BYTES_SIZE,
        })
    }

    fn decode_u32(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U32(peek_u32(bytes)?),
            bytes_read: U32_BYTES_SIZE,
        })
    }

    fn decode_u64(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U64(peek_u64(bytes)?),
            bytes_read: U64_BYTES_SIZE,
        })
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

    fn decode_b256(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::B256(*peek_fixed::<B256_BYTES_SIZE>(bytes)?),
            bytes_read: B256_BYTES_SIZE,
        })
    }

    fn decode_bytes(bytes: &[u8]) -> Result<Decoded> {
        let length = peek_length(bytes)?;
        let bytes = peek(skip(bytes, LENGTH_BYTES_SIZE)?, length)?;

        Ok(Decoded {
            token: Token::Bytes(bytes.to_vec()),
            bytes_read: LENGTH_BYTES_SIZE + bytes.len(),
        })
    }

    fn decode_std_string(bytes: &[u8]) -> Result<Decoded> {
        let length = peek_length(bytes)?;
        let bytes = peek(skip(bytes, LENGTH_BYTES_SIZE)?, length)?;

        Ok(Decoded {
            token: Token::String(str::from_utf8(bytes)?.to_string()),
            bytes_read: LENGTH_BYTES_SIZE + bytes.len(),
        })
    }

    fn decode_raw_slice(bytes: &[u8]) -> Result<Decoded> {
        let length = peek_length(bytes)?;
        let bytes = peek(skip(bytes, LENGTH_BYTES_SIZE)?, length)?;

        Ok(Decoded {
            token: Token::RawSlice(bytes.to_vec()),
            bytes_read: LENGTH_BYTES_SIZE + bytes.len(),
        })
    }

    fn decode_string_array(bytes: &[u8], length: usize) -> Result<Decoded> {
        let bytes = peek(bytes, length)?;
        let decoded = str::from_utf8(bytes)?.to_string();

        Ok(Decoded {
            token: Token::StringArray(StaticStringToken::new(decoded, Some(length))),
            bytes_read: length,
        })
    }

    fn decode_string_slice(bytes: &[u8]) -> Result<Decoded> {
        let length = peek_length(bytes)?;
        let bytes = peek(skip(bytes, LENGTH_BYTES_SIZE)?, length)?;
        let decoded = str::from_utf8(bytes)?.to_string();

        Ok(Decoded {
            token: Token::StringSlice(StaticStringToken::new(decoded, None)),
            bytes_read: bytes.len(),
        })
    }

    fn decode_tuple(&mut self, param_types: &[ParamType], bytes: &[u8]) -> Result<Decoded> {
        let (tokens, bytes_read) = self.decode_params(param_types, bytes)?;

        Ok(Decoded {
            token: Token::Tuple(tokens),
            bytes_read,
        })
    }

    fn decode_array(
        &mut self,
        param_type: &ParamType,
        bytes: &[u8],
        length: usize,
    ) -> Result<Decoded> {
        let (tokens, bytes_read) = self.decode_params(repeat(param_type).take(length), bytes)?;

        Ok(Decoded {
            token: Token::Array(tokens),
            bytes_read,
        })
    }

    fn decode_vector(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Decoded> {
        let length = peek_length(bytes)?;
        let bytes = skip(bytes, LENGTH_BYTES_SIZE)?;
        let (tokens, bytes_read) = self.decode_params(repeat(param_type).take(length), bytes)?;

        Ok(Decoded {
            token: Token::Vector(tokens),
            bytes_read: LENGTH_BYTES_SIZE + bytes_read,
        })
    }

    fn decode_struct(&mut self, param_types: &[ParamType], bytes: &[u8]) -> Result<Decoded> {
        let (tokens, bytes_read) = self.decode_params(param_types, bytes)?;

        Ok(Decoded {
            token: Token::Struct(tokens),
            bytes_read,
        })
    }

    fn decode_enum(&mut self, bytes: &[u8], variants: &EnumVariants) -> Result<Decoded> {
        let discriminant = peek_discriminant(bytes)?;
        let variant_bytes = skip(bytes, DISCRIMINANT_BYTES_SIZE)?;
        let selected_variant = variants.param_type_of_variant(discriminant)?;

        let decoded = self.decode_param(selected_variant, variant_bytes)?;

        Ok(Decoded {
            token: Token::Enum(Box::new((discriminant, decoded.token, variants.clone()))),
            bytes_read: DISCRIMINANT_BYTES_SIZE + decoded.bytes_read,
        })
    }

    fn decode_params<'a>(
        &mut self,
        param_types: impl IntoIterator<Item = &'a ParamType>,
        bytes: &[u8],
    ) -> Result<(Vec<Token>, usize)> {
        let mut tokens = vec![];
        let mut bytes_read = 0;

        for param_type in param_types {
            let decoded = self.decode_param(param_type, skip(bytes, bytes_read)?)?;
            tokens.push(decoded.token);
            bytes_read += decoded.bytes_read;
        }

        Ok((tokens, bytes_read))
    }
}

#[derive(Debug, Clone)]
struct Decoded {
    token: Token,
    bytes_read: usize,
}

struct CounterWithLimit {
    count: usize,
    max: usize,
    name: String,
}

impl CounterWithLimit {
    fn new(max: usize, name: impl Into<String>) -> Self {
        Self {
            count: 0,
            max,
            name: name.into(),
        }
    }

    fn increase(&mut self) -> Result<()> {
        self.count += 1;
        if self.count > self.max {
            return Err(error!(
                Codec,
                "{} limit `{}` reached while decoding. Try increasing it", self.name, self.max
            ));
        }

        Ok(())
    }

    fn decrease(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}

fn peek_u8(bytes: &[u8]) -> Result<u8> {
    let slice = peek_fixed::<U8_BYTES_SIZE>(bytes)?;
    Ok(u8::from_be_bytes(*slice))
}

fn peek_u16(bytes: &[u8]) -> Result<u16> {
    let slice = peek_fixed::<U16_BYTES_SIZE>(bytes)?;
    Ok(u16::from_be_bytes(*slice))
}

fn peek_u32(bytes: &[u8]) -> Result<u32> {
    let slice = peek_fixed::<U32_BYTES_SIZE>(bytes)?;
    Ok(u32::from_be_bytes(*slice))
}

fn peek_u64(bytes: &[u8]) -> Result<u64> {
    let slice = peek_fixed::<U64_BYTES_SIZE>(bytes)?;
    Ok(u64::from_be_bytes(*slice))
}

fn peek_u128(bytes: &[u8]) -> Result<u128> {
    let slice = peek_fixed::<U128_BYTES_SIZE>(bytes)?;
    Ok(u128::from_be_bytes(*slice))
}

fn peek_u256(bytes: &[u8]) -> Result<U256> {
    let slice = peek_fixed::<U256_BYTES_SIZE>(bytes)?;
    Ok(U256::from(*slice))
}

fn peek_length(bytes: &[u8]) -> Result<usize> {
    let slice = peek_fixed::<LENGTH_BYTES_SIZE>(bytes)?;

    u64::from_be_bytes(*slice)
        .try_into()
        .map_err(|_| error!(Other, "could not convert `u64` to `usize`"))
}

fn peek_discriminant(bytes: &[u8]) -> Result<u64> {
    let slice = peek_fixed::<DISCRIMINANT_BYTES_SIZE>(bytes)?;
    Ok(u64::from_be_bytes(*slice))
}

fn peek(data: &[u8], len: usize) -> Result<&[u8]> {
    (len <= data.len()).then_some(&data[..len]).ok_or(error!(
        Codec,
        "tried to read `{len}` bytes but only had `{}` remaining!",
        data.len()
    ))
}

fn peek_fixed<const LEN: usize>(data: &[u8]) -> Result<&[u8; LEN]> {
    let slice_w_correct_length = peek(data, LEN)?;
    Ok(slice_w_correct_length
        .try_into()
        .expect("peek(data, len) must return a slice of length `len` or error out"))
}

fn skip(slice: &[u8], num_bytes: usize) -> Result<&[u8]> {
    (num_bytes <= slice.len())
        .then_some(&slice[num_bytes..])
        .ok_or(error!(
            Codec,
            "tried to consume `{num_bytes}` bytes but only had `{}` remaining!",
            slice.len()
        ))
}
