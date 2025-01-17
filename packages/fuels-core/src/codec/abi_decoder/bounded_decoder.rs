use std::{io::Read, iter::repeat, str};

use crate::{
    codec::{
        utils::{CodecDirection, CounterWithLimit},
        DecoderConfig,
    },
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
}

impl BoundedDecoder {
    pub(crate) fn new(config: DecoderConfig) -> Self {
        let depth_tracker =
            CounterWithLimit::new(config.max_depth, "depth", CodecDirection::Decoding);
        let token_tracker =
            CounterWithLimit::new(config.max_tokens, "token", CodecDirection::Decoding);
        Self {
            depth_tracker,
            token_tracker,
        }
    }

    pub(crate) fn decode<R: Read>(
        &mut self,
        param_type: &ParamType,
        bytes: &mut R,
    ) -> Result<Token> {
        self.decode_param(param_type, bytes)
    }

    pub(crate) fn decode_multiple<R: Read>(
        &mut self,
        param_types: &[ParamType],
        bytes: &mut R,
    ) -> Result<Vec<Token>> {
        self.decode_params(param_types, bytes)
    }

    fn run_w_depth_tracking(
        &mut self,
        decoder: impl FnOnce(&mut Self) -> Result<Token>,
    ) -> Result<Token> {
        self.depth_tracker.increase()?;
        let res = decoder(self);
        self.depth_tracker.decrease();

        res
    }

    fn decode_param<R: Read>(&mut self, param_type: &ParamType, bytes: &mut R) -> Result<Token> {
        self.token_tracker.increase()?;
        match param_type {
            ParamType::Unit => Ok(Token::Unit),
            ParamType::Bool => decode(bytes, |[value]| Token::Bool(value != 0)),
            ParamType::U8 => decode(bytes, |[value]| Token::U8(value)),
            ParamType::U16 => decode(bytes, |value| Token::U16(u16::from_be_bytes(value))),
            ParamType::U32 => decode(bytes, |value| Token::U32(u32::from_be_bytes(value))),
            ParamType::U64 => decode(bytes, |value| Token::U64(u64::from_be_bytes(value))),
            ParamType::U128 => decode(bytes, |value| Token::U128(u128::from_be_bytes(value))),
            ParamType::U256 => decode(bytes, |value| Token::U256(U256::from(value))),
            ParamType::B256 => decode(bytes, Token::B256),
            ParamType::Bytes => Ok(Token::Bytes(decode_slice(bytes)?)),
            ParamType::String => Self::decode_std_string(bytes),
            ParamType::RawSlice => Ok(Token::RawSlice(decode_slice(bytes)?)),
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
            ParamType::Enum { enum_variants, .. } => {
                self.run_w_depth_tracking(|ctx| ctx.decode_enum(enum_variants, bytes))
            }
        }
    }

    fn decode_std_string<R: Read>(bytes: &mut R) -> Result<Token> {
        let data = decode_slice(bytes)?;
        let string = str::from_utf8(&data)?.to_string();
        Ok(Token::String(string))
    }

    fn decode_string_array<R: Read>(bytes: &mut R, length: usize) -> Result<Token> {
        let data = decode_sized(bytes, length)?;
        let decoded = str::from_utf8(&data)?.to_string();
        Ok(Token::StringArray(StaticStringToken::new(
            decoded,
            Some(length),
        )))
    }

    fn decode_string_slice<R: Read>(bytes: &mut R) -> Result<Token> {
        let data = decode_slice(bytes)?;
        let decoded = str::from_utf8(&data)?.to_string();
        Ok(Token::StringSlice(StaticStringToken::new(decoded, None)))
    }

    fn decode_tuple<R: Read>(&mut self, param_types: &[ParamType], bytes: &mut R) -> Result<Token> {
        Ok(Token::Tuple(self.decode_params(param_types, bytes)?))
    }

    fn decode_array<R: Read>(
        &mut self,
        param_type: &ParamType,
        bytes: &mut R,
        length: usize,
    ) -> Result<Token> {
        Ok(Token::Array(
            self.decode_params(repeat(param_type).take(length), bytes)?,
        ))
    }

    fn decode_vector<R: Read>(&mut self, param_type: &ParamType, bytes: &mut R) -> Result<Token> {
        let length = decode_len(bytes)?;
        Ok(Token::Vector(
            self.decode_params(repeat(param_type).take(length), bytes)?,
        ))
    }

    fn decode_struct<R: Read>(
        &mut self,
        fields: &[NamedParamType],
        bytes: &mut R,
    ) -> Result<Token> {
        Ok(Token::Struct(
            self.decode_params(fields.iter().map(|(_, pt)| pt), bytes)?,
        ))
    }

    fn decode_enum<R: Read>(
        &mut self,
        enum_variants: &EnumVariants,
        bytes: &mut R,
    ) -> Result<Token> {
        let discriminant = decode(bytes, u64::from_be_bytes)?;
        let (_, selected_variant) = enum_variants.select_variant(discriminant)?;

        let decoded = self.decode_param(selected_variant, bytes)?;

        Ok(Token::Enum(Box::new((
            discriminant,
            decoded,
            enum_variants.clone(),
        ))))
    }

    fn decode_params<'a, R: Read>(
        &mut self,
        param_types: impl IntoIterator<Item = &'a ParamType>,
        bytes: &mut R,
    ) -> Result<Vec<Token>> {
        let mut tokens = vec![];
        for param_type in param_types {
            tokens.push(self.decode_param(param_type, bytes)?);
        }
        Ok(tokens)
    }
}

/// Decodes a fixed-size array of bytes using a converter function.
fn decode<const SIZE: usize, R: Read, Out>(
    bytes: &mut R,
    f: impl FnOnce([u8; SIZE]) -> Out,
) -> Result<Out> {
    let mut buffer = [0u8; SIZE];
    bytes.read_exact(&mut buffer)?;
    Ok(f(buffer))
}

/// Reads a byte array with known size.
fn decode_sized<R: Read>(bytes: &mut R, len: usize) -> Result<Vec<u8>> {
    let mut data = vec![0; len];
    bytes.read_exact(&mut data)?;
    Ok(data)
}

/// Decodes a length prefix.
fn decode_len<R: Read>(bytes: &mut R) -> Result<usize> {
    let len_u64 = decode(bytes, u64::from_be_bytes)?;
    let len: usize = len_u64
        .try_into()
        .map_err(|_| error!(Other, "could not convert `u64` to `usize`"))?;
    Ok(len)
}

/// Decodes a size-prefixed slice.
fn decode_slice<R: Read>(bytes: &mut R) -> Result<Vec<u8>> {
    let len = decode_len(bytes)?;
    let mut data = vec![0; len];
    bytes.read_exact(&mut data)?;
    Ok(data)
}
