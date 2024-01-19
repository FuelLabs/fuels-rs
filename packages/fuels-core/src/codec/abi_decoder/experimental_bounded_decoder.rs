use std::str;

use crate::{
    checked_round_up_to_word_alignment,
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

const U128_BYTES_SIZE: usize = 2 * WORD_SIZE;
const U256_BYTES_SIZE: usize = 4 * WORD_SIZE;
const B256_BYTES_SIZE: usize = 4 * WORD_SIZE;

impl ExperimentalBoundedDecoder {
    pub(crate) fn new(config: DecoderConfig) -> Self {
        let depth_tracker = CounterWithLimit::new(config.max_depth, "Depth");
        let token_tracker = CounterWithLimit::new(config.max_tokens, "Token");
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
            ParamType::U8 => Self::decode_u8(bytes),
            ParamType::U16 => Self::decode_u16(bytes),
            ParamType::U32 => Self::decode_u32(bytes),
            ParamType::U64 => Self::decode_u64(bytes),
            ParamType::U128 => Self::decode_u128(bytes),
            ParamType::U256 => Self::decode_u256(bytes),
            ParamType::Bool => Self::decode_bool(bytes),
            ParamType::B256 => Self::decode_b256(bytes),
            ParamType::RawSlice => self.decode_raw_slice(bytes),
            ParamType::StringSlice => Self::decode_string_slice(bytes),
            ParamType::StringArray(len) => Self::decode_string_array(bytes, *len),
            ParamType::Array(ref t, length) => {
                self.run_w_depth_tracking(|ctx| ctx.decode_array(t, bytes, *length))
            }
            ParamType::Struct { fields, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_struct(fields, bytes))
            }
            ParamType::Enum { variants, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_enum(bytes, variants))
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
        let num_of_elements = peek_u64(bytes)? as usize;
        let bytes = peek(skip(bytes, WORD_SIZE)?, num_of_elements)?;
        Ok(Decoded {
            token: Token::Bytes(bytes.to_vec()),
            bytes_read: WORD_SIZE + bytes.len(),
        })
    }

    fn decode_std_string(bytes: &[u8]) -> Result<Decoded> {
        let num_of_elements = peek_u64(bytes)? as usize;
        let bytes = peek(skip(bytes, WORD_SIZE)?, num_of_elements)?;
        Ok(Decoded {
            token: Token::String(str::from_utf8(bytes)?.to_string()),
            bytes_read: WORD_SIZE + bytes.len(),
        })
    }

    fn decode_vector(&mut self, param_type: &ParamType, bytes: &[u8]) -> Result<Decoded> {
        let num_of_elements = peek_u64(bytes)? as usize;
        let bytes = skip(bytes, WORD_SIZE)?;
        let (tokens, bytes_read) =
            self.decode_params(std::iter::repeat(param_type).take(num_of_elements), bytes)?;

        Ok(Decoded {
            token: Token::Vector(tokens),
            bytes_read: WORD_SIZE + bytes_read,
        })
    }

    fn decode_tuple(&mut self, param_types: &[ParamType], bytes: &[u8]) -> Result<Decoded> {
        let mut tokens = vec![];

        let mut bytes_read = 0;

        for param_type in param_types.iter() {
            let res = self.decode_param(param_type, skip(bytes, bytes_read)?)?;
            bytes_read += res.bytes_read;
            tokens.push(res.token);
        }

        Ok(Decoded {
            token: Token::Tuple(tokens),
            bytes_read,
        })
    }

    fn decode_struct(&mut self, param_types: &[ParamType], bytes: &[u8]) -> Result<Decoded> {
        let mut tokens = vec![];

        let mut bytes_read = 0;

        for param_type in param_types.iter() {
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

    fn decode_raw_slice(&mut self, bytes: &[u8]) -> Result<Decoded> {
        let num_of_elements = peek_u64(bytes)? as usize;
        let bytes = peek(skip(bytes, WORD_SIZE)?, num_of_elements)?;
        Ok(Decoded {
            token: Token::RawSlice(bytes.to_vec()),
            bytes_read: WORD_SIZE + bytes.len(),
        })
    }

    fn decode_string_slice(bytes: &[u8]) -> Result<Decoded> {
        let size = peek_u64(bytes)? as usize;
        let bytes = peek(skip(bytes, WORD_SIZE)?, size)?;
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
            token: Token::B256(*peek_fixed::<B256_BYTES_SIZE>(bytes)?),
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
            bytes_read: 4,
        })
    }

    fn decode_u16(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U16(peek_u16(bytes)?),
            bytes_read: 2,
        })
    }

    fn decode_u8(bytes: &[u8]) -> Result<Decoded> {
        Ok(Decoded {
            token: Token::U8(peek_u8(bytes)?),
            bytes_read: 1,
        })
    }

    fn decode_unit() -> Result<Decoded> {
        Ok(Decoded {
            token: Token::Unit,
            bytes_read: 0,
        })
    }

    /// The encoding follows the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
    ///
    /// # Arguments
    ///
    /// * `data`: slice of encoded data on whose beginning we're expecting an encoded enum
    /// * `variants`: all types that this particular enum type could hold
    fn decode_enum(&mut self, bytes: &[u8], variants: &EnumVariants) -> Result<Decoded> {
        let discriminant = peek_u64(bytes)?;
        let selected_variant = variants.param_type_of_variant(discriminant)?;

        let enum_content_bytes = skip(bytes, WORD_SIZE)?;
        let result = self.decode_token_in_enum(enum_content_bytes, variants, selected_variant)?;

        let selector = Box::new((discriminant, result.token, variants.clone()));
        Ok(Decoded {
            token: Token::Enum(selector),
            bytes_read: WORD_SIZE + result.bytes_read,
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
            Err(error!(
                InvalidType,
                "{} limit ({}) reached while decoding. Try increasing it.", self.name, self.max
            ))
        } else {
            Ok(())
        }
    }

    fn decrease(&mut self) {
        if self.count > 0 {
            self.count -= 1;
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
    let slice = peek_fixed::<4>(bytes)?;
    Ok(u32::from_be_bytes(*slice))
}

fn peek_u16(bytes: &[u8]) -> Result<u16> {
    let slice = peek_fixed::<2>(bytes)?;
    Ok(u16::from_be_bytes(*slice))
}

fn peek_u8(bytes: &[u8]) -> Result<u8> {
    let slice = peek_fixed::<1>(bytes)?;
    Ok(u8::from_be_bytes(*slice))
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
