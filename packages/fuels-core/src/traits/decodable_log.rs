use std::iter::zip;

use fuels_types::{errors::Error, param_types::ParamType, Token};

use crate::abi_decoder::ABIDecoder;

pub trait DecodableLog {
    fn decode_log(&self, data: &[u8]) -> Result<String, Error>;
}

impl DecodableLog for ParamType {
    fn decode_log(&self, data: &[u8]) -> Result<String, Error> {
        let token = ABIDecoder::decode_single(self, data)?;
        paramtype_decode_log(self, &token)
    }
}

fn inner_types_log(
    tokens: &[Token],
    inner_type: &ParamType,
    join_str: &str,
) -> Result<String, Error> {
    let inner_types_log = tokens
        .iter()
        .map(|token| paramtype_decode_log(inner_type, token))
        .collect::<Result<Vec<_>, _>>()?
        .join(join_str);

    Ok(inner_types_log)
}

fn paramtype_decode_log(param_type: &ParamType, token: &Token) -> Result<String, Error> {
    let result = match (param_type, token) {
        (ParamType::U8, Token::U8(val)) => val.to_string(),
        (ParamType::U16, Token::U16(val)) => val.to_string(),
        (ParamType::U32, Token::U32(val)) => val.to_string(),
        (ParamType::U64, Token::U64(val)) => val.to_string(),
        (ParamType::Bool, Token::Bool(val)) => val.to_string(),
        (ParamType::Byte, Token::Byte(val)) => val.to_string(),
        (ParamType::B256, Token::B256(val)) => {
            format!("Bits256({val:?})")
        }
        (ParamType::Unit, Token::Unit) => "()".to_string(),
        (ParamType::String(..), Token::String(str_token)) => {
            format!(
                "SizedAsciiString {{ data: \"{}\" }}",
                str_token.get_encodable_str()?
            )
        }
        (ParamType::Array(inner_type, _), Token::Array(tokens)) => {
            let elements = inner_types_log(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Vector(inner_type), Token::Vector(tokens)) => {
            let elements = inner_types_log(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Struct { name, fields, .. }, Token::Struct(field_tokens)) => {
            let fields = zip(fields, field_tokens)
                .map(|((field_name, param_type), token)| -> Result<_, Error> {
                    let field_stringified = paramtype_decode_log(param_type, token)?;
                    Ok(format!("{field_name}: {}", field_stringified))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            format!("{name} {{ {fields} }}")
        }
        (ParamType::Enum { .. }, Token::Enum(selector)) => {
            let (discriminant, token, variants) = selector.as_ref();

            let (variant_name, variant_param_type) = variants.select_variant(*discriminant)?;
            let variant_str = paramtype_decode_log(variant_param_type, token)?;
            let variant_str = if variant_str == "()" {
                "".into()
            } else {
                format!("({variant_str})")
            };

            format!("{variant_name}{variant_str}")
        }
        (ParamType::Tuple(types), Token::Tuple(tokens)) => {
            let elements = zip(types, tokens)
                .map(|(ptype, token)| paramtype_decode_log(ptype, token))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");

            format!("({elements})")
        }
        _ => {
            return Err(Error::InvalidData(format!(
                "Could not decode log with param type: `{param_type:?}` and token: `{token:?}`"
            )))
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use fuels_types::{errors::Error, Bits256, EvmAddress, SizedAsciiString};

    use crate::{traits::DecodableLog, Parameterize};

    #[test]
    fn test_param_type_decode_log() -> Result<(), Error> {
        {
            assert_eq!(
                format!("{:?}", true),
                bool::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 0, 1])?
            );

            assert_eq!(
                format!("{:?}", 128u8),
                u8::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 0, 128])?
            );

            assert_eq!(
                format!("{:?}", 256u16),
                u16::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 1, 0])?
            );

            assert_eq!(
                format!("{:?}", 512u32),
                u32::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 2, 0])?
            );

            assert_eq!(
                format!("{:?}", 1024u64),
                u64::param_type().decode_log(&[0, 0, 0, 0, 0, 0, 4, 0])?
            );
        }
        {
            assert_eq!(
                format!("{:?}", (1, 2)),
                <(u8, u8)>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2])?
            );

            assert_eq!(
                format!("{:?}", [3, 4]),
                <[u64; 2]>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4])?
            );

            assert_eq!(
                format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
                SizedAsciiString::<4>::param_type().decode_log(&[70, 117, 101, 108, 0, 0, 0, 0])?
            );
        }
        {
            assert_eq!(
                format!("{:?}", Some(42)),
                <Option<u64>>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42])?
            );

            assert_eq!(
                format!("{:?}", Err::<u64, u64>(42u64)),
                <Result<u64, u64>>::param_type()
                    .decode_log(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42])?
            );

            let bits256 = Bits256([
                239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161,
                16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
            ]);

            assert_eq!(
                format!("{:?}", bits256),
                Bits256::param_type().decode_log(&[
                    239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89,
                    161, 16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                ])?
            );

            assert_eq!(
                format!("{:?}", EvmAddress::from(bits256)),
                EvmAddress::param_type().decode_log(&[
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 166, 225, 89, 161, 16, 60, 239, 183,
                    226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                ])?
            );
        }

        Ok(())
    }
}
