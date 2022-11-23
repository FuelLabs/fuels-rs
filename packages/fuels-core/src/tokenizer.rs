use crate::{unzip_param_types, StringToken, Token};
use fuels_types::{errors::Error, param_types::ParamType, utils::has_array_format};
use hex::FromHex;

#[derive(Default)]
pub struct Tokenizer;

impl Tokenizer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tokenizer {
    /// Takes a ParamType and a value string and joins them as a single
    /// Token that holds the value within it. This Token is used
    /// in the encoding process.
    pub fn tokenize(param: &ParamType, value: String) -> Result<Token, Error> {
        if !value.is_ascii() {
            return Err(Error::InvalidData(
                "value string can only contain ascii characters".into(),
            ));
        }

        let trimmed_value = value.trim();

        match param {
            ParamType::Unit => Ok(Token::Unit),
            ParamType::U8 => Ok(Token::U8(trimmed_value.parse::<u8>()?)),
            ParamType::U16 => Ok(Token::U16(trimmed_value.parse::<u16>()?)),
            ParamType::U32 => Ok(Token::U32(trimmed_value.parse::<u32>()?)),
            ParamType::U64 => Ok(Token::U64(trimmed_value.parse::<u64>()?)),
            ParamType::Bool => Ok(Token::Bool(trimmed_value.parse::<bool>()?)),
            ParamType::Byte => Ok(Token::Byte(trimmed_value.parse::<u8>()?)),
            ParamType::B256 => {
                const B256_HEX_ENC_LENGTH: usize = 64;
                if trimmed_value.len() != B256_HEX_ENC_LENGTH {
                    return Err(Error::InvalidData(format!(
                        "the hex encoding of the b256 must have {} characters",
                        B256_HEX_ENC_LENGTH
                    )));
                }
                let v = Vec::from_hex(trimmed_value)?;
                let s: [u8; 32] = v.as_slice().try_into().unwrap();
                Ok(Token::B256(s))
            }
            ParamType::Vector(param_type) => Self::tokenize_vec(trimmed_value, param_type),
            ParamType::Array(t, _) => Ok(Self::tokenize_array(trimmed_value, t)?),
            ParamType::String(length) => Ok(Token::String(StringToken::new(
                trimmed_value.into(),
                *length,
            ))),
            ParamType::Struct {
                fields: struct_params,
                ..
            } => Ok(Self::tokenize_struct(
                trimmed_value,
                &unzip_param_types(struct_params),
            )?),
            ParamType::Enum { variants, .. } => {
                let discriminant = get_enum_discriminant_from_string(trimmed_value);
                let value = get_enum_value_from_string(trimmed_value);

                let token = Self::tokenize(&variants.param_types()[discriminant], value)?;

                Ok(Token::Enum(Box::new((
                    discriminant as u8,
                    token,
                    variants.clone(),
                ))))
            }
            ParamType::Tuple(tuple_params) => {
                Ok(Self::tokenize_tuple(trimmed_value, tuple_params)?)
            }
        }
    }

    /// Creates a `Token::Struct` from an array of parameter types and a string of values.
    /// I.e. it takes a string containing values "value_1, value_2, value_3" and an array
    /// of `ParamType` containing the type of each value, in order:
    /// [ParamType::<Type of value_1>, ParamType::<Type of value_2>, ParamType::<Type of value_3>]
    /// And attempts to return a `Token::Struct()` containing the inner types.
    /// It works for nested/recursive structs.
    pub fn tokenize_struct(value: &str, params: &[ParamType]) -> Result<Token, Error> {
        if !value.starts_with('(') || !value.ends_with(')') {
            return Err(Error::InvalidData(
                "struct value string must start and end with round brackets".into(),
            ));
        }

        if value.chars().count() == 2 {
            return Ok(Token::Struct(vec![]));
        }

        // To parse the value string, we use a two-pointer/index approach.
        // The items are comma-separated, and if an item is tokenized, the last_item
        // index is moved to the current position.
        // The variable nested is incremented and decremented if a bracket is encountered,
        // and appropriate errors are returned if the nested count is not 0.
        // If the struct has an array inside its values, the current position will be incremented
        // until the opening and closing bracket are inside the new item.
        // Characters inside quotes are ignored, and they are tokenized as one item.
        // An error is returned if there is an odd number of quotes.
        let mut result = vec![];
        let mut nested = 0isize;
        let mut ignore = false;
        let mut last_item = 1;
        let mut params_iter = params.iter();

        for (pos, ch) in value.chars().enumerate() {
            match ch {
                '(' if !ignore => {
                    nested += 1;
                }
                ')' if !ignore => {
                    nested -= 1;

                    match nested.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            return Err(Error::InvalidData(
                                "struct value string has excess closing brackets".into(),
                            ));
                        }
                        std::cmp::Ordering::Equal => {
                            let sub = &value[last_item..pos];

                            let token = Self::tokenize(
                                params_iter.next().ok_or_else(|| {
                                    Error::InvalidData(
                                        "struct value contains more elements than the parameter types provided".into(),
                                    )
                                })?,
                                sub.to_string(),
                            )?;
                            result.push(token);
                            last_item = pos + 1;
                        }
                        _ => {}
                    }
                }
                '"' => {
                    ignore = !ignore;
                }
                ',' if nested == 1 && !ignore => {
                    let sub = &value[last_item..pos];
                    // If we've encountered an array within a struct property
                    // keep iterating until we see the end of it "]".
                    if sub.contains('[') && !sub.contains(']') {
                        continue;
                    }

                    let token = Self::tokenize(
                        params_iter.next().ok_or_else(|| {
                            Error::InvalidData(
                                "struct value contains more elements than the parameter types provided".into(),
                            )
                        })?,
                        sub.to_string(),
                    )?;
                    result.push(token);
                    last_item = pos + 1;
                }
                _ => (),
            }
        }

        if ignore {
            return Err(Error::InvalidData(
                "struct value string has excess quotes".into(),
            ));
        }

        if nested > 0 {
            return Err(Error::InvalidData(
                "struct value string has excess opening brackets".into(),
            ));
        }

        Ok(Token::Struct(result))
    }

    /// Creates a `Token::Array` from one parameter type and a string of values. I.e. it takes a
    /// string containing values "value_1, value_2, value_3" and a `ParamType` specifying the type.
    /// It works for nested/recursive arrays.
    pub fn tokenize_array(value: &str, param: &ParamType) -> Result<Token, Error> {
        let result = Self::extract_multiple(&value, param)?;

        Ok(Token::Array(result))
    }

    pub fn tokenize_vec(value: &str, param: &ParamType) -> Result<Token, Error> {
        let result = Self::extract_multiple(&value, param)?;

        Ok(Token::Vector(result))
    }

    fn extract_multiple(value: &&str, param: &ParamType) -> Result<Vec<Token>, Error> {
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(Error::InvalidData(
                "array/vec value string must start and end with square brackets".into(),
            ));
        }

        if value.chars().count() == 2 {
            return Ok(vec![]);
        }

        // For more details about this algorithm, refer to the tokenize_struct method.
        let mut result = vec![];
        let mut nested = 0isize;
        let mut ignore = false;
        let mut last_item = 1;
        for (i, ch) in value.chars().enumerate() {
            match ch {
                '[' if !ignore => {
                    nested += 1;
                }
                ']' if !ignore => {
                    nested -= 1;

                    match nested.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            return Err(Error::InvalidData(
                                "array/vec value string has excess closing brackets".into(),
                            ));
                        }
                        std::cmp::Ordering::Equal => {
                            // Last element of this nest level; proceed to tokenize.
                            let sub = &value[last_item..i];
                            match has_array_format(sub) {
                                true => {
                                    let arr_param = ParamType::Array(
                                        Box::new(param.to_owned()),
                                        get_array_length_from_string(sub),
                                    );

                                    result.push(Self::tokenize(&arr_param, sub.to_string())?);
                                }
                                false => {
                                    result.push(Self::tokenize(param, sub.to_string())?);
                                }
                            }

                            last_item = i + 1;
                        }
                        _ => {}
                    }
                }
                '"' => {
                    ignore = !ignore;
                }
                ',' if nested == 1 && !ignore => {
                    let sub = &value[last_item..i];
                    match has_array_format(sub) {
                        true => {
                            let arr_param = ParamType::Array(
                                Box::new(param.to_owned()),
                                get_array_length_from_string(sub),
                            );

                            result.push(Self::tokenize(&arr_param, sub.to_string())?);
                        }
                        false => {
                            result.push(Self::tokenize(param, sub.to_string())?);
                        }
                    }
                    last_item = i + 1;
                }
                _ => (),
            }
        }

        if ignore {
            return Err(Error::InvalidData(
                "array/vec value string has excess quotes".into(),
            ));
        }

        if nested > 0 {
            return Err(Error::InvalidData(
                "array/vec value string has excess opening brackets".into(),
            ));
        }
        Ok(result)
    }

    /// Creates `Token::Tuple` from an array of parameter types and a string of values.
    /// I.e. it takes a string containing values "value_1, value_2, value_3" and an array
    /// of `ParamType` containing the type of each value, in order:
    /// [ParamType::<Type of value_1>, ParamType::<Type of value_2>, ParamType::<Type of value_3>]
    /// And attempts to return a `Token::Tuple()` containing the inner types.
    /// It works for nested/recursive tuples.
    pub fn tokenize_tuple(value: &str, params: &[ParamType]) -> Result<Token, Error> {
        if !value.starts_with('(') || !value.ends_with(')') {
            return Err(Error::InvalidData(
                "tuple value string must start and end with round brackets".into(),
            ));
        }

        if value.chars().count() == 2 {
            return Ok(Token::Tuple(vec![]));
        }

        // For more details about this algorithm, refer to the tokenize_struct method.
        let mut result = vec![];
        let mut nested = 0isize;
        let mut ignore = false;
        let mut last_item = 1;
        let mut params_iter = params.iter();

        for (pos, ch) in value.chars().enumerate() {
            match ch {
                '(' if !ignore => {
                    nested += 1;
                }
                ')' if !ignore => {
                    nested -= 1;

                    match nested.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            return Err(Error::InvalidData(
                                "tuple value string has excess closing brackets".into(),
                            ));
                        }
                        std::cmp::Ordering::Equal => {
                            let sub = &value[last_item..pos];

                            let token = Self::tokenize(
                                params_iter.next().ok_or_else(|| {
                                    Error::InvalidData(
                                        "tuple value contains more elements than the parameter types provided".into(),
                                    )
                                })?,
                                sub.to_string(),
                            )?;
                            result.push(token);
                            last_item = pos + 1;
                        }
                        _ => {}
                    }
                }
                '"' => {
                    ignore = !ignore;
                }
                ',' if nested == 1 && !ignore => {
                    let sub = &value[last_item..pos];
                    // If we've encountered an array within a tuple property
                    // keep iterating until we see the end of it "]".
                    if sub.contains('[') && !sub.contains(']') {
                        continue;
                    }

                    let token = Self::tokenize(
                        params_iter.next().ok_or_else(|| {
                            Error::InvalidData(
                                "tuple value contains more elements than the parameter types provided".into(),
                            )
                        })?,
                        sub.to_string(),
                    )?;
                    result.push(token);
                    last_item = pos + 1;
                }
                _ => (),
            }
        }

        if ignore {
            return Err(Error::InvalidData(
                "tuple value string has excess quotes".into(),
            ));
        }

        if nested > 0 {
            return Err(Error::InvalidData(
                "tuple value string has excess opening brackets".into(),
            ));
        }

        Ok(Token::Tuple(result))
    }
}
fn get_enum_discriminant_from_string(ele: &str) -> usize {
    let mut chars = ele.chars();
    chars.next(); // Remove "("
    chars.next_back(); // Remove ")"
    let v: Vec<_> = chars.as_str().split(',').collect();
    v[0].parse().unwrap()
}

fn get_enum_value_from_string(ele: &str) -> String {
    let mut chars = ele.chars();
    chars.next(); // Remove "("
    chars.next_back(); // Remove ")"
    let v: Vec<_> = chars.as_str().split(',').collect();
    v[1].to_string()
}

fn get_array_length_from_string(ele: &str) -> usize {
    let mut chars = ele.chars();
    chars.next();
    chars.next_back();
    chars.as_str().split(',').count()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizable;

    #[test]
    fn tokenize_struct_excess_value_elements_expected_error() -> Result<(), Error> {
        let struct_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_struct("(0, [0,0,0], 0, 0)", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value contains more elements than the parameter types provided",
            error_message
        );

        let error_message = Tokenizer::tokenize_struct("(0, [0,0,0], 0)", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value contains more elements than the parameter types provided",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_struct_excess_quotes_expected_error() -> Result<(), Error> {
        let struct_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_struct("(0, \"[0,0,0])", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value string has excess quotes",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_struct_invalid_start_end_bracket_expected_error() -> Result<(), Error> {
        let struct_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_struct("0, [0,0,0])", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value string must start and end with round brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_struct_excess_opening_bracket_expected_error() -> Result<(), Error> {
        let struct_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_struct("((0, [0,0,0])", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value string has excess opening brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_struct_excess_closing_bracket_expected_error() -> Result<(), Error> {
        let struct_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_struct("(0, [0,0,0]))", &struct_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: struct value string has excess closing brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_uint_types_expected_error() {
        // We test only on U8 as it is the same error on all other unsigned int types
        let error_message = Tokenizer::tokenize(&ParamType::U8, "2,".to_string())
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Parse integer error: invalid digit found in string",
            error_message
        );
    }

    #[test]
    fn tokenize_bool_expected_error() {
        let error_message = Tokenizer::tokenize(&ParamType::Bool, "True".to_string())
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Parse boolean error: provided string was not `true` or `false`",
            error_message
        );
    }

    #[test]
    fn tokenize_b256_invalid_length_expected_error() {
        let value = "d57a9c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e90b".to_string();
        let error_message = Tokenizer::tokenize(&ParamType::B256, value)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: the hex encoding of the b256 must have 64 characters",
            error_message
        );
    }

    #[test]
    fn tokenize_b256_invalid_character_expected_error() {
        let value = "Hd57a9c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e90b".to_string();
        let error_message = Tokenizer::tokenize(&ParamType::B256, value)
            .unwrap_err()
            .to_string();

        assert!(error_message.contains("Parse hex error: Invalid character"));
    }

    #[test]
    fn tokenize_tuple_invalid_start_end_bracket_expected_error() -> Result<(), Error> {
        let tuple_params = [ParamType::Tuple(vec![
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ])];
        let error_message = Tokenizer::tokenize_tuple("0, [0,0,0])", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value string must start and end with round brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_tuple_excess_opening_bracket_expected_error() -> Result<(), Error> {
        let tuple_params = [ParamType::Tuple(vec![
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ])];
        let error_message = Tokenizer::tokenize_tuple("((0, [0,0,0])", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value string has excess opening brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_tuple_excess_closing_bracket_expected_error() -> Result<(), Error> {
        let tuple_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_tuple("(0, [0,0,0]))", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value string has excess closing brackets",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_tuple_excess_quotes_expected_error() -> Result<(), Error> {
        let tuple_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_tuple("(0, \"[0,0,0])", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value string has excess quotes",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_tuple_excess_value_elements_expected_error() -> Result<(), Error> {
        let tuple_params = [
            ParamType::U64,
            ParamType::Array(Box::new(ParamType::U64), 3),
        ];
        let error_message = Tokenizer::tokenize_tuple("(0, [0,0,0], 0, 0)", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value contains more elements than the parameter types provided",
            error_message
        );

        let error_message = Tokenizer::tokenize_tuple("(0, [0,0,0], 0)", &tuple_params)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: tuple value contains more elements than the parameter types provided",
            error_message
        );
        Ok(())
    }

    #[test]
    fn tokenize_array_invalid_start_end_bracket_expected_error() {
        let param = ParamType::U16;

        let error_message = Tokenizer::tokenize_array("1,2],[3],4]", &param)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: array/vec value string must start and end with square brackets",
            error_message
        );
    }

    #[test]
    fn tokenize_array_excess_opening_bracket_expected_error() {
        let param = ParamType::U16;

        let error_message = Tokenizer::tokenize_array("[[[1,2],[3],4]", &param)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: array/vec value string has excess opening brackets",
            error_message
        );
    }

    #[test]
    fn tokenize_array_excess_closing_bracket_expected_error() {
        let param = ParamType::U16;

        let error_message = Tokenizer::tokenize_array("[[1,2],[3],4]]", &param)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: array/vec value string has excess closing brackets",
            error_message
        );
    }

    #[test]
    fn tokenize_array_excess_quotes_expected_error() {
        let param = ParamType::U16;
        let error_message = Tokenizer::tokenize_array("[[1,\"2],[3],4]]", &param)
            .unwrap_err()
            .to_string();

        assert_eq!(
            "Invalid data: array/vec value string has excess quotes",
            error_message
        );
    }

    #[test]
    fn tokenize_array() -> Result<(), Error> {
        let value = "[[1,2],[3],4]";
        let param = ParamType::U16;
        let tokens = Tokenizer::tokenize_array(value, &param)?;

        let expected_tokens = Token::Array(vec![
            Token::Array(vec![Token::U16(1), Token::U16(2)]), // First element, a sub-array with 2 elements
            Token::Array(vec![Token::U16(3)]), // Second element, a sub-array with 1 element
            Token::U16(4),                     // Third element
        ]);

        assert_eq!(tokens, expected_tokens);

        let value = "[1,[2],[3],[4,5]]";
        let param = ParamType::U16;
        let tokens = Tokenizer::tokenize_array(value, &param)?;

        let expected_tokens = Token::Array(vec![
            Token::U16(1),
            Token::Array(vec![Token::U16(2)]),
            Token::Array(vec![Token::U16(3)]),
            Token::Array(vec![Token::U16(4), Token::U16(5)]),
        ]);

        assert_eq!(tokens, expected_tokens);

        let value = "[1,2,3,4,5]";
        let param = ParamType::U16;
        let tokens = Tokenizer::tokenize_array(value, &param)?;

        let expected_tokens = Token::Array(vec![
            Token::U16(1),
            Token::U16(2),
            Token::U16(3),
            Token::U16(4),
            Token::U16(5),
        ]);

        assert_eq!(tokens, expected_tokens);

        let value = "[[1,2,3,[4,5]]]";
        let param = ParamType::U16;
        let tokens = Tokenizer::tokenize_array(value, &param)?;

        let expected_tokens = Token::Array(vec![Token::Array(vec![
            Token::U16(1),
            Token::U16(2),
            Token::U16(3),
            Token::Array(vec![Token::U16(4), Token::U16(5)]),
        ])]);

        assert_eq!(tokens, expected_tokens);
        Ok(())
    }

    #[test]
    fn tokenize_vec() -> Result<(), Error> {
        let param_type = ParamType::Vector(Box::new(ParamType::U8));
        let input = "[1,2,3]".to_string();

        let result = Tokenizer::tokenize(&param_type, input)?;

        let the_vec = Vec::<u8>::from_token(result)?;

        assert_eq!(the_vec, vec![1, 2, 3]);

        Ok(())
    }
}
