use std::iter::zip;

use crate::types::{
    errors::{error, Result},
    param_types::ParamType,
    Token,
};

fn inner_types_debug(tokens: &[Token], inner_type: &ParamType, join_str: &str) -> Result<String> {
    let inner_types_log = tokens
        .iter()
        .map(|token| decode_as_debug_str(inner_type, token))
        .collect::<Result<Vec<_>>>()?
        .join(join_str);

    Ok(inner_types_log)
}

pub(crate) fn decode_as_debug_str(param_type: &ParamType, token: &Token) -> Result<String> {
    let result = match (param_type, token) {
        (ParamType::Unit, Token::Unit) => "()".to_string(),
        (ParamType::Bool, Token::Bool(val)) => val.to_string(),
        (ParamType::U8, Token::U8(val)) => val.to_string(),
        (ParamType::U16, Token::U16(val)) => val.to_string(),
        (ParamType::U32, Token::U32(val)) => val.to_string(),
        (ParamType::U64, Token::U64(val)) => val.to_string(),
        (ParamType::U128, Token::U128(val)) => val.to_string(),
        (ParamType::U256, Token::U256(val)) => val.to_string(),
        (ParamType::B256, Token::B256(val)) => {
            format!("Bits256({val:?})")
        }
        (ParamType::Bytes, Token::Bytes(val)) => {
            format!("Bytes({val:?})")
        }
        (ParamType::String, Token::String(val)) => val.clone(),
        (ParamType::RawSlice, Token::RawSlice(val)) => {
            format!("RawSlice({val:?})")
        }
        (ParamType::StringArray(..), Token::StringArray(str_token)) => {
            format!("SizedAsciiString {{ data: \"{}\" }}", str_token.data)
        }
        (ParamType::StringSlice, Token::StringSlice(str_token)) => {
            format!("AsciiString {{ data: \"{}\" }}", str_token.data)
        }
        (ParamType::Tuple(types), Token::Tuple(tokens)) => {
            let elements = zip(types, tokens)
                .map(|(ptype, token)| decode_as_debug_str(ptype, token))
                .collect::<Result<Vec<_>>>()?
                .join(", ");

            format!("({elements})")
        }
        (ParamType::Array(inner_type, _), Token::Array(tokens)) => {
            let elements = inner_types_debug(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Vector(inner_type), Token::Vector(tokens)) => {
            let elements = inner_types_debug(tokens, inner_type, ", ")?;
            format!("[{elements}]")
        }
        (ParamType::Struct { name, fields, .. }, Token::Struct(field_tokens)) => {
            let fields = zip(fields, field_tokens)
                .map(|((field_name, param_type), token)| -> Result<_> {
                    Ok(format!(
                        "{field_name}: {}",
                        decode_as_debug_str(param_type, token)?
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            format!("{name} {{ {fields} }}")
        }
        (ParamType::Enum { .. }, Token::Enum(selector)) => {
            let (discriminant, token, variants) = selector.as_ref();

            let (variant_name, variant_param_type) = variants.select_variant(*discriminant)?;
            let variant_str = decode_as_debug_str(variant_param_type, token)?;
            let variant_str = if variant_str == "()" {
                "".into()
            } else {
                format!("({variant_str})")
            };

            format!("{variant_name}{variant_str}")
        }
        _ => {
            return Err(error!(
                Codec,
                "could not decode debug from param type: `{param_type:?}` and token: `{token:?}`"
            ))
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{
        codec::ABIDecoder,
        traits::Parameterize,
        types::{
            errors::Result, AsciiString, Bits256, Bytes, EvmAddress, RawSlice, SizedAsciiString,
            U256,
        },
    };

    #[test]
    fn param_type_decode_debug() -> Result<()> {
        let decoder = ABIDecoder::default();
        {
            assert_eq!(
                format!("{:?}", true),
                decoder.decode_as_debug_str(&bool::param_type(), [1].as_slice())?
            );

            assert_eq!(
                format!("{:?}", 128u8),
                decoder.decode_as_debug_str(&u8::param_type(), [128].as_slice())?
            );

            assert_eq!(
                format!("{:?}", 256u16),
                decoder.decode_as_debug_str(&u16::param_type(), [1, 0].as_slice())?
            );

            assert_eq!(
                format!("{:?}", 512u32),
                decoder.decode_as_debug_str(&u32::param_type(), [0, 0, 2, 0].as_slice())?
            );

            assert_eq!(
                format!("{:?}", 1024u64),
                decoder
                    .decode_as_debug_str(&u64::param_type(), [0, 0, 0, 0, 0, 0, 4, 0].as_slice())?
            );

            assert_eq!(
                format!("{:?}", 1024u128),
                decoder.decode_as_debug_str(
                    &u128::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0].as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", U256::from(2048)),
                decoder.decode_as_debug_str(
                    &U256::param_type(),
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 8, 0
                    ]
                    .as_slice()
                )?
            );
        }
        {
            let bytes = [
                239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89, 161,
                16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74,
            ];
            let bits256 = Bits256(bytes);

            assert_eq!(
                format!("{bits256:?}"),
                decoder.decode_as_debug_str(
                    &Bits256::param_type(),
                    [
                        239, 134, 175, 169, 105, 108, 240, 220, 99, 133, 226, 196, 7, 166, 225, 89,
                        161, 16, 60, 239, 183, 226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                    ]
                    .as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", Bytes(bytes.to_vec())),
                decoder.decode_as_debug_str(
                    &Bytes::param_type(),
                    [
                        0, 0, 0, 0, 0, 0, 0, 32, 239, 134, 175, 169, 105, 108, 240, 220, 99, 133,
                        226, 196, 7, 166, 225, 89, 161, 16, 60, 239, 183, 226, 174, 6, 54, 251, 51,
                        211, 203, 42, 158, 74
                    ]
                    .as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", RawSlice(bytes.to_vec())),
                decoder.decode_as_debug_str(
                    &RawSlice::param_type(),
                    [
                        0, 0, 0, 0, 0, 0, 0, 32, 239, 134, 175, 169, 105, 108, 240, 220, 99, 133,
                        226, 196, 7, 166, 225, 89, 161, 16, 60, 239, 183, 226, 174, 6, 54, 251, 51,
                        211, 203, 42, 158, 74
                    ]
                    .as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", EvmAddress::from(bits256)),
                decoder.decode_as_debug_str(
                    &EvmAddress::param_type(),
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 166, 225, 89, 161, 16, 60, 239, 183,
                        226, 174, 6, 54, 251, 51, 211, 203, 42, 158, 74
                    ]
                    .as_slice()
                )?
            );
        }
        {
            assert_eq!(
                format!("{:?}", AsciiString::new("Fuel".to_string())?),
                decoder.decode_as_debug_str(
                    &AsciiString::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 4, 70, 117, 101, 108].as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", SizedAsciiString::<4>::new("Fuel".to_string())?),
                decoder.decode_as_debug_str(
                    &SizedAsciiString::<4>::param_type(),
                    [70, 117, 101, 108, 0, 0, 0, 0].as_slice()
                )?
            );

            assert_eq!(
                format!("{}", "Fuel"),
                decoder.decode_as_debug_str(
                    &String::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 4, 70, 117, 101, 108].as_slice()
                )?
            );
        }
        {
            assert_eq!(
                format!("{:?}", (1, 2)),
                decoder.decode_as_debug_str(&<(u8, u8)>::param_type(), [1, 2].as_slice())?
            );

            assert_eq!(
                format!("{:?}", [3, 4]),
                decoder.decode_as_debug_str(
                    &<[u64; 2]>::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4].as_slice()
                )?
            );
        }
        {
            assert_eq!(
                format!("{:?}", Some(42)),
                decoder.decode_as_debug_str(
                    &<Option<u64>>::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42].as_slice()
                )?
            );

            assert_eq!(
                format!("{:?}", Err::<u64, u64>(42u64)),
                decoder.decode_as_debug_str(
                    &<std::result::Result<u64, u64>>::param_type(),
                    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 42].as_slice()
                )?
            );
        }

        Ok(())
    }
}
