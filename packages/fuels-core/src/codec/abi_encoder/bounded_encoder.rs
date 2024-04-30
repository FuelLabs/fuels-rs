use crate::{
    codec::{
        utils::{CodecDirection, CounterWithLimit},
        EncoderConfig,
    },
    types::{
        errors::Result,
        unresolved_bytes::{Data, UnresolvedBytes},
        EnumSelector, StaticStringToken, Token, U256,
    },
};

pub(crate) struct BoundedEncoder {
    depth_tracker: CounterWithLimit,
    token_tracker: CounterWithLimit,
}

impl BoundedEncoder {
    pub(crate) fn new(config: EncoderConfig, _unused: bool) -> Self {
        let depth_tracker =
            CounterWithLimit::new(config.max_depth, "depth", CodecDirection::Encoding);
        let token_tracker =
            CounterWithLimit::new(config.max_tokens, "token", CodecDirection::Encoding);
        Self {
            depth_tracker,
            token_tracker,
        }
    }

    pub fn encode(&mut self, args: &[Token]) -> Result<UnresolvedBytes> {
        let data = vec![Data::Inline(self.encode_tokens(args)?)];

        Ok(UnresolvedBytes::new(data))
    }

    fn encode_tokens(&mut self, tokens: &[Token]) -> Result<Vec<u8>> {
        let mut data = vec![];

        for token in tokens.iter() {
            let new_data = self.encode_token(token)?;
            data.extend(new_data);
        }

        Ok(data)
    }

    fn run_w_depth_tracking(
        &mut self,
        encoder: impl FnOnce(&mut Self) -> Result<Vec<u8>>,
    ) -> Result<Vec<u8>> {
        self.depth_tracker.increase()?;
        let res = encoder(self);
        self.depth_tracker.decrease();

        res
    }

    fn encode_token(&mut self, arg: &Token) -> Result<Vec<u8>> {
        self.token_tracker.increase()?;
        let encoded_token = match arg {
            Token::Unit => vec![],
            Token::Bool(arg_bool) => vec![u8::from(*arg_bool)],
            Token::U8(arg_u8) => vec![*arg_u8],
            Token::U16(arg_u16) => arg_u16.to_be_bytes().to_vec(),
            Token::U32(arg_u32) => arg_u32.to_be_bytes().to_vec(),
            Token::U64(arg_u64) => arg_u64.to_be_bytes().to_vec(),
            Token::U128(arg_u128) => arg_u128.to_be_bytes().to_vec(),
            Token::U256(arg_u256) => Self::encode_u256(*arg_u256),
            Token::B256(arg_bits256) => arg_bits256.to_vec(),
            Token::Bytes(data) => Self::encode_bytes(data.to_vec())?,
            Token::String(string) => Self::encode_bytes(string.clone().into_bytes())?,
            Token::RawSlice(data) => Self::encode_bytes(data.clone())?,
            Token::StringArray(arg_string) => Self::encode_string_array(arg_string)?,
            Token::StringSlice(arg_string) => Self::encode_string_slice(arg_string)?,
            Token::Tuple(arg_tuple) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_tokens(arg_tuple))?
            }
            Token::Array(arg_array) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_tokens(arg_array))?
            }
            Token::Vector(data) => self.run_w_depth_tracking(|ctx| ctx.encode_vector(data))?,
            Token::Struct(arg_struct) => {
                self.run_w_depth_tracking(|ctx| ctx.encode_tokens(arg_struct))?
            }
            Token::Enum(arg_enum) => self.run_w_depth_tracking(|ctx| ctx.encode_enum(arg_enum))?,
        };

        Ok(encoded_token)
    }

    fn encode_u256(arg_u256: U256) -> Vec<u8> {
        let mut bytes = [0u8; 32];
        arg_u256.to_big_endian(&mut bytes);

        bytes.to_vec()
    }

    fn encode_bytes(data: Vec<u8>) -> Result<Vec<u8>> {
        let len = data.len();

        Ok([Self::encode_length(len as u64), data].concat())
    }

    fn encode_string_array(arg_string: &StaticStringToken) -> Result<Vec<u8>> {
        Ok(arg_string.get_encodable_str()?.as_bytes().to_vec())
    }

    fn encode_string_slice(arg_string: &StaticStringToken) -> Result<Vec<u8>> {
        Self::encode_bytes(arg_string.get_encodable_str()?.as_bytes().to_vec())
    }

    fn encode_vector(&mut self, data: &[Token]) -> Result<Vec<u8>> {
        let encoded_data = self.encode_tokens(data)?;

        Ok([Self::encode_length(data.len() as u64), encoded_data].concat())
    }

    fn encode_enum(&mut self, selector: &EnumSelector) -> Result<Vec<u8>> {
        let (discriminant, token_within_enum, _) = selector;
        let encoded_discriminant = Self::encode_discriminant(*discriminant);
        let encoded_token = self.encode_token(token_within_enum)?;

        Ok([encoded_discriminant, encoded_token].concat())
    }

    fn encode_length(len: u64) -> Vec<u8> {
        len.to_be_bytes().to_vec()
    }

    fn encode_discriminant(discriminant: u64) -> Vec<u8> {
        discriminant.to_be_bytes().to_vec()
    }
}
