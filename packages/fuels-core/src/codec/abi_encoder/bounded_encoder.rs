use fuel_types::bytes::padded_len_usize;

use crate::{
    checked_round_up_to_word_alignment,
    codec::{
        utils::{CodecDirection, CounterWithLimit},
        EncoderConfig,
    },
    error,
    types::{
        errors::Result,
        pad_u16, pad_u32,
        unresolved_bytes::{Data, UnresolvedBytes},
        EnumSelector, StaticStringToken, Token, U256,
    },
};

pub(crate) struct BoundedEncoder {
    used_for_configurables: bool,
    depth_tracker: CounterWithLimit,
    token_tracker: CounterWithLimit,
    max_total_enum_width: usize,
}

impl BoundedEncoder {
    pub(crate) fn new(config: EncoderConfig, used_for_configurables: bool) -> Self {
        let depth_tracker =
            CounterWithLimit::new(config.max_depth, "depth", CodecDirection::Encoding);
        let token_tracker =
            CounterWithLimit::new(config.max_tokens, "token", CodecDirection::Encoding);
        Self {
            depth_tracker,
            token_tracker,
            max_total_enum_width: config.max_total_enum_width,
            used_for_configurables,
        }
    }

    /// Encodes `Token`s in `args` following the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md)
    pub fn encode(&mut self, args: &[Token]) -> Result<UnresolvedBytes> {
        // Checking that the tokens can be encoded is not done here, because it would require
        // going through the whole array of tokens, which can be pretty inefficient.
        let data = if args.len() == 1 {
            match args[0] {
                Token::U8(arg_u8) if self.used_for_configurables => {
                    vec![Self::encode_u8_as_byte(arg_u8)]
                }
                Token::U8(arg_u8) => vec![Self::encode_u8_as_u64(arg_u8)],
                Token::Bool(arg_bool) if self.used_for_configurables => {
                    vec![Self::encode_bool_as_byte(arg_bool)]
                }
                Token::Bool(arg_bool) => {
                    vec![Self::encode_bool_as_u64(arg_bool)]
                }
                _ => self.encode_tokens(args, true)?,
            }
        } else {
            self.encode_tokens(args, true)?
        };

        Ok(UnresolvedBytes::new(data))
    }

    fn encode_tokens(&mut self, tokens: &[Token], word_aligned: bool) -> Result<Vec<Data>> {
        let mut offset_in_bytes = 0;
        let mut data = vec![];

        for token in tokens {
            self.token_tracker.increase()?;
            let mut new_data = self.encode_token(token)?;
            offset_in_bytes += new_data.iter().map(Data::size_in_bytes).sum::<usize>();

            data.append(&mut new_data);

            if word_aligned {
                let padding = vec![
                    0u8;
                    checked_round_up_to_word_alignment(offset_in_bytes)?
                        - offset_in_bytes
                ];
                if !padding.is_empty() {
                    offset_in_bytes += padding.len();
                    data.push(Data::Inline(padding));
                }
            }
        }

        Ok(data)
    }

    fn run_w_depth_tracking(
        &mut self,
        encoder: impl FnOnce(&mut Self) -> Result<Vec<Data>>,
    ) -> Result<Vec<Data>> {
        self.depth_tracker.increase()?;

        let res = encoder(self);

        self.depth_tracker.decrease();
        res
    }

    fn encode_token(&mut self, arg: &Token) -> Result<Vec<Data>> {
        let encoded_token = match arg {
            Token::Unit => vec![Self::encode_unit()],
            Token::U8(arg_u8) => vec![Self::encode_u8_as_byte(*arg_u8)],
            Token::U16(arg_u16) => vec![Self::encode_u16(*arg_u16)],
            Token::U32(arg_u32) => vec![Self::encode_u32(*arg_u32)],
            Token::U64(arg_u64) => vec![Self::encode_u64(*arg_u64)],
            Token::U128(arg_u128) => vec![Self::encode_u128(*arg_u128)],
            Token::U256(arg_u256) => vec![Self::encode_u256(*arg_u256)],
            Token::Bool(arg_bool) => vec![Self::encode_bool_as_byte(*arg_bool)],
            Token::B256(arg_bits256) => vec![Self::encode_b256(arg_bits256)],
            Token::RawSlice(data) => Self::encode_raw_slice(data.clone())?,
            Token::StringSlice(arg_string) => Self::encode_string_slice(arg_string)?,
            Token::StringArray(arg_string) => vec![Self::encode_string_array(arg_string)?],
            Token::Array(arg_array) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_array(arg_array))?
            }
            Token::Struct(arg_struct) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_struct(arg_struct))?
            }
            Token::Enum(arg_enum) => self.run_w_depth_tracking(|ctx| ctx.encode_enum(arg_enum))?,
            Token::Tuple(arg_tuple) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_tuple(arg_tuple))?
            }
            Token::Vector(data) => self.run_w_depth_tracking(|ctx| ctx.encode_vector(data))?,
            Token::Bytes(data) => Self::encode_bytes(data.to_vec())?,
            // `String` in Sway has the same memory layout as the bytes type
            Token::String(string) => Self::encode_bytes(string.clone().into_bytes())?,
        };

        Ok(encoded_token)
    }

    fn encode_unit() -> Data {
        Data::Inline(vec![0u8])
    }

    fn encode_tuple(&mut self, arg_tuple: &[Token]) -> Result<Vec<Data>> {
        self.encode_tokens(arg_tuple, true)
    }

    fn encode_struct(&mut self, subcomponents: &[Token]) -> Result<Vec<Data>> {
        self.encode_tokens(subcomponents, true)
    }

    fn encode_array(&mut self, arg_array: &[Token]) -> Result<Vec<Data>> {
        self.encode_tokens(arg_array, false)
    }

    fn encode_b256(arg_bits256: &[u8; 32]) -> Data {
        Data::Inline(arg_bits256.to_vec())
    }

    fn encode_bool_as_byte(arg_bool: bool) -> Data {
        Data::Inline(vec![u8::from(arg_bool)])
    }

    fn encode_bool_as_u64(arg_bool: bool) -> Data {
        Data::Inline(vec![0, 0, 0, 0, 0, 0, 0, u8::from(arg_bool)])
    }

    fn encode_u128(arg_u128: u128) -> Data {
        Data::Inline(arg_u128.to_be_bytes().to_vec())
    }

    fn encode_u256(arg_u256: U256) -> Data {
        let mut bytes = [0u8; 32];
        arg_u256.to_big_endian(&mut bytes);
        Data::Inline(bytes.to_vec())
    }

    fn encode_u64(arg_u64: u64) -> Data {
        Data::Inline(arg_u64.to_be_bytes().to_vec())
    }

    fn encode_u32(arg_u32: u32) -> Data {
        Data::Inline(pad_u32(arg_u32).to_vec())
    }

    fn encode_u16(arg_u16: u16) -> Data {
        Data::Inline(pad_u16(arg_u16).to_vec())
    }

    fn encode_u8_as_byte(arg_u8: u8) -> Data {
        Data::Inline(vec![arg_u8])
    }

    fn encode_u8_as_u64(arg_u8: u8) -> Data {
        Data::Inline(vec![0, 0, 0, 0, 0, 0, 0, arg_u8])
    }

    fn encode_enum(&mut self, selector: &EnumSelector) -> Result<Vec<Data>> {
        let (discriminant, token_within_enum, variants) = selector;

        let mut encoded_enum = vec![Self::encode_discriminant(*discriminant)];

        // Enums that contain only Units as variants have only their discriminant encoded.
        if !variants.only_units_inside() {
            let (_, variant_param_type) = variants.select_variant(*discriminant)?;
            let enum_width_in_bytes = variants.compute_enum_width_in_bytes()?;

            if enum_width_in_bytes > self.max_total_enum_width {
                return Err(error!(
                    Codec,
                    "cannot encode enum with variants: {variants:?}. It is `{enum_width_in_bytes}` bytes wide. Try increasing maximum total enum width."
                ));
            }
            let padding_amount = variants.compute_padding_amount_in_bytes(variant_param_type)?;

            encoded_enum.push(Data::Inline(vec![0; padding_amount]));

            let token_data = self.encode_token(token_within_enum)?;
            encoded_enum.extend(token_data);
        }

        Ok(encoded_enum)
    }

    fn encode_discriminant(discriminant: u64) -> Data {
        Self::encode_u64(discriminant)
    }

    fn encode_vector(&mut self, data: &[Token]) -> Result<Vec<Data>> {
        let encoded_data = self.encode_tokens(data, false)?;
        let cap = data.len() as u64;
        let len = data.len() as u64;

        // A vector is expected to be encoded as 3 WORDs -- a ptr, a cap and a
        // len. This means that we must place the encoded vector elements
        // somewhere else. Hence the use of Data::Dynamic which will, when
        // resolved, leave behind in its place only a pointer to the actual
        // data.
        Ok(vec![
            Data::Dynamic(encoded_data),
            Self::encode_u64(cap),
            Self::encode_u64(len),
        ])
    }

    fn encode_raw_slice(mut data: Vec<u8>) -> Result<Vec<Data>> {
        let len = data.len();

        zeropad_to_word_alignment(&mut data);

        let encoded_data = vec![Data::Inline(data)];

        Ok(vec![
            Data::Dynamic(encoded_data),
            Self::encode_u64(len as u64),
        ])
    }

    fn encode_string_slice(arg_string: &StaticStringToken) -> Result<Vec<Data>> {
        let encodable_str = arg_string.get_encodable_str()?;

        let encoded_data = Data::Inline(encodable_str.as_bytes().to_vec());
        let len = Self::encode_u64(encodable_str.len() as u64);

        Ok(vec![Data::Dynamic(vec![encoded_data]), len])
    }

    fn encode_string_array(arg_string: &StaticStringToken) -> Result<Data> {
        Ok(Data::Inline(crate::types::pad_string(
            arg_string.get_encodable_str()?,
        )))
    }

    fn encode_bytes(mut data: Vec<u8>) -> Result<Vec<Data>> {
        let len = data.len();

        zeropad_to_word_alignment(&mut data);

        let cap = data.len() as u64;
        let encoded_data = vec![Data::Inline(data)];

        Ok(vec![
            Data::Dynamic(encoded_data),
            Self::encode_u64(cap),
            Self::encode_u64(len as u64),
        ])
    }
}

fn zeropad_to_word_alignment(data: &mut Vec<u8>) {
    let padded_length = padded_len_usize(data.len());
    data.resize(padded_length, 0);
}
