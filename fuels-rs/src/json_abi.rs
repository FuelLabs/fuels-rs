use crate::{abi_decoder::ABIDecoder, abi_encoder::ABIEncoder, errors::Error};
use fuels_core::{ParamType, Token};
use hex::FromHex;
use itertools::Itertools;
use std::convert::TryInto;
use std::str;
use std::str::FromStr;

use core_types::{JsonABI, Property};

use serde_json;

pub struct ABIParser {
    fn_selector: Option<Vec<u8>>,
}

impl ABIParser {
    pub fn new() -> Self {
        ABIParser { fn_selector: None }
    }

    /// Higher-level layer of the ABI encoding module.
    /// Encode is essentially a wrapper of [`crate::abi_encoder`],
    /// but it is responsible for parsing strings into proper [`Token`]
    /// that can be encoded by the [`crate::abi_encoder`].
    /// Note that `encode` only encodes the parameters for an ABI call,
    /// It won't include the function selector in it. To get the function
    /// selector, use `encode_with_function_selector`.
    ///
    /// # Examples
    /// ```
    /// use fuels_rs::json_abi::ABIParser;
    /// let json_abi = r#"
    ///     [
    ///         {
    ///             "type":"contract",
    ///             "inputs":[
    ///                 {
    ///                     "name":"arg",
    ///                     "type":"u32"
    ///                 }
    ///             ],
    ///             "name":"takes_u32_returns_bool",
    ///             "outputs":[
    ///                 {
    ///                     "name":"",
    ///                     "type":"bool"
    ///                 }
    ///             ]
    ///         }
    ///     ]
    ///     "#;
    ///
    ///     let values: Vec<String> = vec!["10".to_string()];
    ///
    ///     let mut abi = ABIParser::new();
    ///
    ///     let function_name = "takes_u32_returns_bool";
    ///     let encoded = abi.encode(json_abi, function_name, &values).unwrap();
    ///     let expected_encode = "000000000000000a";
    ///     assert_eq!(encoded, expected_encode);
    /// ```
    pub fn encode(&mut self, abi: &str, fn_name: &str, values: &[String]) -> Result<String, Error> {
        let parsed_abi: JsonABI = serde_json::from_str(abi)?;

        let entry = parsed_abi.iter().find(|e| e.name == fn_name);

        if entry.is_none() {
            return Err(Error::InvalidName(format!(
                "couldn't find function name: {}",
                fn_name
            )));
        }

        let entry = entry.unwrap();

        let mut encoder = ABIEncoder::new_with_fn_selector(
            self.build_fn_selector(fn_name, &entry.inputs)?.as_bytes(),
        );

        // Update the fn_selector field with the encoded selector.
        self.fn_selector = Some(encoder.function_selector.to_vec());

        let params: Vec<_> = entry
            .inputs
            .iter()
            .map(|param| parse_param(param).unwrap())
            .zip(values.iter().map(|v| v as &str))
            .collect();

        let tokens = self.parse_tokens(&params)?;

        Ok(hex::encode(encoder.encode(&tokens)?))
    }

    /// Similar to `encode`, but includes the function selector in the
    /// final encoded string.
    ///
    /// # Examples
    /// ```
    /// use fuels_rs::json_abi::ABIParser;
    /// let json_abi = r#"
    ///     [
    ///         {
    ///             "type":"contract",
    ///             "inputs":[
    ///                 {
    ///                     "name":"arg",
    ///                     "type":"u32"
    ///                 }
    ///             ],
    ///             "name":"takes_u32_returns_bool",
    ///             "outputs":[
    ///                 {
    ///                     "name":"",
    ///                     "type":"bool"
    ///                 }
    ///             ]
    ///         }
    ///     ]
    ///     "#;
    ///
    ///     let values: Vec<String> = vec!["10".to_string()];
    ///
    ///     let mut abi = ABIParser::new();

    ///     let function_name = "takes_u32_returns_bool";
    ///
    ///     let encoded = abi
    ///         .encode_with_function_selector(json_abi, function_name, &values)
    ///         .unwrap();
    ///
    ///     let expected_encode = "000000006355e6ee000000000000000a";
    ///     assert_eq!(encoded, expected_encode);
    /// ```
    pub fn encode_with_function_selector(
        &mut self,
        abi: &str,
        fn_name: &str,
        values: &[String],
    ) -> Result<String, Error> {
        let encoded_params = self.encode(abi, fn_name, values)?;
        let fn_selector = self
            .fn_selector
            .to_owned()
            .expect("Function selector not encoded");

        let encoded_fn_selector = hex::encode(fn_selector);

        Ok(format!("{}{}", encoded_fn_selector, encoded_params))
    }

    /// Helper function to return the encoded function selector.
    /// It must already be encoded.
    pub fn get_encoded_function_selector(&self) -> String {
        let fn_selector = self
            .fn_selector
            .to_owned()
            .expect("Function selector not encoded");

        hex::encode(fn_selector)
    }

    /// Similar to `encode`, but it encodes only an array of strings containing
    /// [<type_1>, <param_1>, <type_2>, <param_2>, <type_n>, <param_n>]
    /// Without having to reference to a JSON specification of the ABI.
    pub fn encode_params(&self, params: &[String]) -> Result<String, Error> {
        let pairs: Vec<_> = params.chunks(2).collect_vec();

        let mut param_type_pairs: Vec<(ParamType, &str)> = vec![];

        let mut encoder = ABIEncoder::new();

        for pair in pairs {
            let prop = Property {
                name: "".to_string(),
                type_field: pair[0].clone(),
                components: None,
            };
            let p = parse_param(&prop)?;

            let t: (ParamType, &str) = (p, &pair[1]);
            param_type_pairs.push(t);
        }

        let tokens = self.parse_tokens(&param_type_pairs)?;

        let encoded = encoder.encode(&tokens)?;

        Ok(hex::encode(encoded))
    }

    /// Helper function to turn a list of tuples(ParamType, &str) into
    /// a vector of Tokens ready to be encoded.
    /// Essentially a wrapper on `tokenize`.
    pub fn parse_tokens<'a>(&self, params: &'a [(ParamType, &str)]) -> Result<Vec<Token>, Error> {
        params
            .iter()
            .map(|&(ref param, value)| self.tokenize(param, value.to_string()))
            .collect::<Result<_, _>>()
            .map_err(From::from)
    }

    /// Takes a ParamType and a value string and joins them as a single
    /// Token that holds the value within it. This Token is used
    /// in the encoding process.
    pub fn tokenize<'a>(&self, param: &ParamType, value: String) -> Result<Token, Error> {
        let trimmed_value = value.trim();
        match &*param {
            ParamType::U8 => Ok(Token::U8(trimmed_value.parse::<u8>()?)),
            ParamType::U16 => Ok(Token::U16(trimmed_value.parse::<u16>()?)),
            ParamType::U32 => Ok(Token::U32(trimmed_value.parse::<u32>()?)),
            ParamType::U64 => Ok(Token::U64(trimmed_value.parse::<u64>()?)),
            ParamType::Bool => Ok(Token::Bool(trimmed_value.parse::<bool>()?)),
            ParamType::Byte => Ok(Token::Byte(trimmed_value.parse::<u8>()?)),
            ParamType::B256 => {
                let v = Vec::from_hex(trimmed_value)?;
                let s: [u8; 32] = v.as_slice().try_into().unwrap();
                Ok(Token::B256(s))
            }
            ParamType::Array(t, _) => Ok(self.tokenize_array(trimmed_value, &*t)?),
            ParamType::String(_) => Ok(Token::String(trimmed_value.to_string())),
            ParamType::Struct(struct_params) => {
                Ok(self.tokenize_struct(trimmed_value, struct_params)?)
            }
            ParamType::Enum(s) => {
                let discriminant = self.get_enum_discriminant_from_string(&value);
                let value = self.get_enum_value_from_string(&value);

                let token = self.tokenize(&s[discriminant], value)?;

                Ok(Token::Enum(Box::new((discriminant as u8, token))))
            }
        }
    }

    /// Creates a struct `Token` from an array of parameter types and a string of values.
    /// I.e. it takes a string containing values "value_1, value_2, value_3" and an array
    /// of `ParamType` containing the type of each value, in order:
    /// [ParamType::<Type of value_1>, ParamType::<Type of value_2>, ParamType::<Type of value_3>]
    /// And attempts to return a `Token::Struct()` containing the inner types.
    /// It works for nested/recursive structs.
    pub fn tokenize_struct(&self, value: &str, params: &[ParamType]) -> Result<Token, Error> {
        if !value.starts_with('(') || !value.ends_with(')') {
            return Err(Error::InvalidData);
        }

        if value.chars().count() == 2 {
            return Ok(Token::Struct(vec![]));
        }

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
                            return Err(Error::InvalidData);
                        }
                        std::cmp::Ordering::Equal => {
                            let sub = &value[last_item..pos];

                            let token = self.tokenize(
                                params_iter.next().ok_or(Error::InvalidData)?,
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

                    let token = self.tokenize(
                        params_iter.next().ok_or(Error::InvalidData)?,
                        sub.to_string(),
                    )?;
                    result.push(token);
                    last_item = pos + 1;
                }
                _ => (),
            }
        }

        if ignore {
            return Err(Error::InvalidData);
        }

        Ok(Token::Struct(result))
    }

    /// Creates an enum `Token` from an array of parameter types and a string of values.
    /// I.e. it takes a string containing values "value_1, value_2, value_3" and an array
    /// of `ParamType` containing the type of each value, in order:
    /// [ParamType::<Type of value_1>, ParamType::<Type of value_2>, ParamType::<Type of value_3>]
    /// And attempts to return a `Token::Enum()` containing the inner types.
    /// It works for nested/recursive enums.
    pub fn tokenize_array<'a>(&self, value: &'a str, param: &ParamType) -> Result<Token, Error> {
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(Error::InvalidData);
        }

        if value.chars().count() == 2 {
            return Ok(Token::Array(vec![]));
        }

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
                            return Err(Error::InvalidData);
                        }
                        std::cmp::Ordering::Equal => {
                            // Last element of this nest level; proceed to tokenize.
                            let sub = &value[last_item..i];
                            match self.is_array(sub) {
                                true => {
                                    let arr_param = ParamType::Array(
                                        Box::new(param.to_owned()),
                                        self.get_array_length_from_string(sub),
                                    );

                                    result.push(self.tokenize(&arr_param, sub.to_string())?);
                                }
                                false => {
                                    result.push(self.tokenize(param, sub.to_string())?);
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
                    match self.is_array(sub) {
                        true => {
                            let arr_param = ParamType::Array(
                                Box::new(param.to_owned()),
                                self.get_array_length_from_string(sub),
                            );

                            result.push(self.tokenize(&arr_param, sub.to_string())?);
                        }
                        false => {
                            result.push(self.tokenize(param, sub.to_string())?);
                        }
                    }
                    last_item = i + 1;
                }
                _ => (),
            }
        }

        if ignore {
            return Err(Error::InvalidData);
        }

        Ok(Token::Array(result))
    }

    /// Higher-level layer of the ABI decoding module.
    /// Decodes a value of a given ABI and a target function's output.
    /// Note that the `value` has to be a byte array, meaning that
    /// the caller must properly cast the "upper" type into a `&[u8]`,
    pub fn decode<'a>(
        &self,
        abi: &str,
        fn_name: &str,
        value: &'a [u8],
    ) -> Result<Vec<Token>, Error> {
        let parsed_abi: JsonABI = serde_json::from_str(abi)?;

        let entry = parsed_abi.iter().find(|e| e.name == fn_name);

        if entry.is_none() {
            return Err(Error::InvalidName(format!(
                "couldn't find function name: {}",
                fn_name
            )));
        }

        let params_result: Result<Vec<_>, _> = entry
            .unwrap()
            .outputs
            .iter()
            .map(|param| parse_param(param))
            .collect();

        match params_result {
            Ok(params) => {
                let mut decoder = ABIDecoder::new();

                Ok(decoder.decode(&params, value)?)
            }
            Err(e) => Err(e),
        }
    }

    /// Similar to decode, but it decodes only an array types and the encoded data
    /// without having to reference to a JSON specification of the ABI.
    pub fn decode_params(&self, params: &[ParamType], data: &[u8]) -> Result<Vec<Token>, Error> {
        let mut decoder = ABIDecoder::new();
        Ok(decoder.decode(params, data)?)
    }

    fn is_array(&self, ele: &str) -> bool {
        ele.starts_with('[') && ele.ends_with(']')
    }

    fn get_enum_discriminant_from_string(&self, ele: &str) -> usize {
        let mut chars = ele.chars();
        chars.next(); // Remove "("
        chars.next_back(); // Remove ")"
        let v: Vec<_> = chars.as_str().split(',').collect();
        v[0].parse().unwrap()
    }

    fn get_enum_value_from_string(&self, ele: &str) -> String {
        let mut chars = ele.chars();
        chars.next(); // Remove "("
        chars.next_back(); // Remove ")"
        let v: Vec<_> = chars.as_str().split(',').collect();
        v[1].to_string()
    }

    fn get_array_length_from_string(&self, ele: &str) -> usize {
        let mut chars = ele.chars();
        chars.next();
        chars.next_back();
        chars.as_str().split(',').count()
    }

    /// Builds a string representation of a function selector,
    /// i.e: <fn_name>(<type_1>, <type_2>, ..., <type_n>)
    pub fn build_fn_selector(&self, fn_name: &str, params: &[Property]) -> Result<String, Error> {
        let fn_selector = fn_name.to_owned();

        let mut result: String = format!("{}(", fn_selector);

        for (idx, param) in params.iter().enumerate() {
            result.push_str(&self.build_fn_selector_params(param));
            if idx + 1 < params.len() {
                result.push(',');
            }
        }

        result.push(')');

        Ok(result)
    }

    fn build_fn_selector_params(&self, param: &Property) -> String {
        let mut result: String = String::new();

        if param.type_field.contains("struct ") || param.type_field.contains("enum ") {
            // Custom type, need to break down inner fields
            // Will return `"s(field_1,field_2,...,field_n)"`.
            result.push_str("s(");

            for (idx, component) in param.components.as_ref().unwrap().iter().enumerate() {
                let res = self.build_fn_selector_params(component);
                result.push_str(&res);

                if idx + 1 < param.components.as_ref().unwrap().len() {
                    result.push(',');
                }
            }
            result.push(')');
        } else {
            result.push_str(&param.type_field);
        }
        result
    }
}

/// Turns a JSON property into ParamType
pub fn parse_param(param: &Property) -> Result<ParamType, Error> {
    match ParamType::from_str(&param.type_field) {
        // Simple case (primitive types, no arrays, including string)
        Ok(param_type) => Ok(param_type),
        Err(_) => {
            match param.type_field.contains("struct") || param.type_field.contains("enum") {
                true => Ok(parse_custom_type_param(param)?),
                false => {
                    match param.type_field.contains('[') && param.type_field.contains(']') {
                        // Try to parse array (T[M]) or string (str[M])
                        true => Ok(parse_array_param(param)?),
                        // Try to parse enum or struct
                        false => Ok(parse_custom_type_param(param)?),
                    }
                }
            }
        }
    }
}

pub fn parse_array_param(param: &Property) -> Result<ParamType, Error> {
    // Split "T[n]" string into "T" and "[n]"
    let split: Vec<&str> = param.type_field.split('[').collect();
    if split.len() != 2 {
        return Err(Error::MissingData(format!(
            "invalid parameter type: {}",
            param.type_field
        )));
    }

    let param_type = ParamType::from_str(split[0]).unwrap();

    // Grab size in between brackets, i.e the `n` in "[n]"
    let size: usize = split[1][..split[1].len() - 1].parse().unwrap();

    if let ParamType::String(_) = param_type {
        Ok(ParamType::String(size))
    } else {
        Ok(ParamType::Array(Box::new(param_type), size))
    }
}

pub fn parse_custom_type_param(param: &Property) -> Result<ParamType, Error> {
    let mut params: Vec<ParamType> = vec![];

    match param.components.as_ref() {
        Some(components) => {
            for component in components {
                params.push(parse_param(component)?)
            }
        }
        None => {
            return Err(Error::MissingData(
                "cannot parse custom type with no components".into(),
            ))
        }
    }

    if param.type_field.contains("struct") {
        return Ok(ParamType::Struct(params));
    }
    if param.type_field.contains("enum") {
        return Ok(ParamType::Enum(params));
    }
    Err(Error::InvalidType(param.type_field.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_encode_and_decode_no_selector() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u32"
                    }
                ],
                "name":"takes_u32_returns_bool",
                "outputs":[
                    {
                        "name":"",
                        "type":"bool"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["10".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_u32_returns_bool";

        let encoded = abi.encode(json_abi, function_name, &values).unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "000000000000000a";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // false
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let expected_return = vec![Token::Bool(false)];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn simple_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u32"
                    }
                ],
                "name":"takes_u32_returns_bool",
                "outputs":[
                    {
                        "name":"",
                        "type":"bool"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["10".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_u32_returns_bool";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "000000006355e6ee000000000000000a";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // false
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let expected_return = vec![Token::Bool(false)];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn b256_and_single_byte_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"foo",
                        "type":"b256"
                    },
                    {
                        "name":"bar",
                        "type":"byte"
                    }
                ],
                "name":"my_func",
                "outputs":[
                    {
                        "name":"",
                        "type":"b256"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec![
            "d5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b".to_string(),
            "1".to_string(),
        ];

        let mut abi = ABIParser::new();

        let function_name = "my_func";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "00000000e64019abd5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b0000000000000001";
        assert_eq!(encoded, expected_encode);

        let return_value =
            hex::decode("a441b15fe9a3cf56661190a0b93b9dec7d04127288cc87250967cf3b52894d11")
                .unwrap();

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let s: [u8; 32] = return_value.as_slice().try_into().unwrap();

        let expected_return = vec![Token::B256(s)];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn array_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u16[3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[
                    {
                        "name":"",
                        "type":"u16[2]"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["[1,2,3]".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_array";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "00000000f0b87864000000000000000100000000000000020000000000000003";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // 0
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // 1
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let expected_return = vec![Token::Array(vec![Token::U16(0), Token::U16(1)])];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn tokenize_array() {
        let abi = ABIParser::new();

        let value = "[[1,2],[3],4]";
        let param = ParamType::U16;
        let tokens = abi.tokenize_array(value, &param).unwrap();

        let expected_tokens = Token::Array(vec![
            Token::Array(vec![Token::U16(1), Token::U16(2)]), // First element, a sub-array with 2 elements
            Token::Array(vec![Token::U16(3)]), // Second element, a sub-array with 1 element
            Token::U16(4),                     // Third element
        ]);

        assert_eq!(tokens, expected_tokens);

        let value = "[1,[2],[3],[4,5]]";
        let param = ParamType::U16;
        let tokens = abi.tokenize_array(value, &param).unwrap();

        let expected_tokens = Token::Array(vec![
            Token::U16(1),
            Token::Array(vec![Token::U16(2)]),
            Token::Array(vec![Token::U16(3)]),
            Token::Array(vec![Token::U16(4), Token::U16(5)]),
        ]);

        assert_eq!(tokens, expected_tokens);

        let value = "[1,2,3,4,5]";
        let param = ParamType::U16;
        let tokens = abi.tokenize_array(value, &param).unwrap();

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
        let tokens = abi.tokenize_array(value, &param).unwrap();

        let expected_tokens = Token::Array(vec![Token::Array(vec![
            Token::U16(1),
            Token::U16(2),
            Token::U16(3),
            Token::Array(vec![Token::U16(4), Token::U16(5)]),
        ])]);

        assert_eq!(tokens, expected_tokens);
    }

    #[test]
    fn nested_array_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u16[3]"
                    }
                ],
                "name":"takes_nested_array",
                "outputs":[
                    {
                        "name":"",
                        "type":"u16[2]"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["[[1,2],[3],[4]]".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_nested_array";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode =
            "00000000e5d521030000000000000001000000000000000200000000000000030000000000000004";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // 0
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // 1
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let expected_return = vec![Token::Array(vec![Token::U16(0), Token::U16(1)])];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn string_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"foo",
                        "type":"str[23]"
                    }
                ],
                "name":"takes_string",
                "outputs":[
                    {
                        "name":"",
                        "type":"str[2]"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["This is a full sentence".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_string";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "00000000d56e76515468697320697320612066756c6c2073656e74656e636500";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x4f, 0x4b, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // "OK" encoded in utf8
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value).unwrap();

        let expected_return = vec![Token::String("OK".into())];

        assert_eq!(decoded_return, expected_return);
    }

    #[test]
    fn struct_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"my_struct",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_struct",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["(42, true)".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_struct";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "00000000cb0b2f05000000000000002a0000000000000001";
        assert_eq!(encoded, expected_encode);
    }

    #[test]
    fn struct_and_primitive_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"my_struct",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    },
                    {
                        "name":"foo",
                        "type":"u32"
                    }
                ],
                "name":"takes_struct_and_primitive",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["(42, true)".to_string(), "10".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_struct_and_primitive";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "000000005c445838000000000000002a0000000000000001000000000000000a";
        assert_eq!(encoded, expected_encode);
    }

    #[test]
    fn nested_struct_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"top_value",
                        "type":"struct MyNestedStruct",
                        "components": [
                            {
                                "name": "x",
                                "type": "u16"
                            },
                            {
                                "name": "inner",
                                "type": "struct Y",
                                "components": [
                                    {
                                        "name":"a",
                                        "type": "bool"
                                    },
                                    {
                                        "name":"b",
                                        "type": "u8[2]"
                                    }
                                ]
                            }
                        ]
                    }
                ],
                "name":"takes_nested_struct",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["(10, (true, [1,2]))".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_nested_struct";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode =
            "00000000ff25eb48000000000000000a000000000000000100000000000000010000000000000002";
        assert_eq!(encoded, expected_encode);

        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"top_value",
                        "type":"struct MyNestedStruct",
                        "components": [
                            {
                                "name": "inner",
                                "type": "struct X",
                                "components": [
                                    {
                                        "name":"a",
                                        "type": "bool"
                                    },
                                    {
                                        "name":"b",
                                        "type": "u8[2]"
                                    }
                                ]
                            },
                            {
                                "name": "y",
                                "type": "u16"
                            }
                        ]
                    }
                ],
                "name":"takes_nested_struct",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["((true, [1,2]), 10)".to_string()];

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode =
            "000000007728cb9e000000000000000100000000000000010000000000000002000000000000000a";
        assert_eq!(encoded, expected_encode);
    }

    #[test]
    fn enum_encode_and_decode() {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"my_enum",
                        "type":"enum MyEnum",
                        "components": [
                            {
                                "name": "x",
                                "type": "u32"
                            },
                            {
                                "name": "y",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_enum",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["(0, 42)".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_enum";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "00000000082e0dfa0000000000000000000000000000002a";
        assert_eq!(encoded, expected_encode);
    }

    #[test]
    fn fn_selector_single_primitive() {
        let abi = ABIParser::new();
        let p = Property {
            name: "foo".into(),
            type_field: "u64".into(),
            components: None,
        };
        let params = vec![p];
        let selector = abi.build_fn_selector("my_func", &params).unwrap();

        assert_eq!(selector, "my_func(u64)");
    }

    #[test]
    fn fn_selector_multiple_primitives() {
        let abi = ABIParser::new();
        let p1 = Property {
            name: "foo".into(),
            type_field: "u64".into(),
            components: None,
        };
        let p2 = Property {
            name: "bar".into(),
            type_field: "bool".into(),
            components: None,
        };
        let params = vec![p1, p2];
        let selector = abi.build_fn_selector("my_func", &params).unwrap();

        assert_eq!(selector, "my_func(u64,bool)");
    }

    #[test]
    fn fn_selector_custom_type() {
        let abi = ABIParser::new();

        let inner_foo = Property {
            name: "foo".into(),
            type_field: "bool".into(),
            components: None,
        };

        let inner_bar = Property {
            name: "bar".into(),
            type_field: "u64".into(),
            components: None,
        };

        let p = Property {
            name: "my_struct".into(),
            type_field: "struct MyStruct".into(),
            components: Some(vec![inner_foo, inner_bar]),
        };

        let params = vec![p];
        let selector = abi.build_fn_selector("my_func", &params).unwrap();

        assert_eq!(selector, "my_func(s(bool,u64))");
    }

    #[test]
    fn fn_selector_nested_custom_type() {
        let abi = ABIParser::new();

        let inner_foo = Property {
            name: "foo".into(),
            type_field: "bool".into(),
            components: None,
        };

        let inner_a = Property {
            name: "a".into(),
            type_field: "u64".into(),
            components: None,
        };

        let inner_b = Property {
            name: "b".into(),
            type_field: "u32".into(),
            components: None,
        };

        let inner_bar = Property {
            name: "bar".into(),
            type_field: "struct InnerStruct".into(),
            components: Some(vec![inner_a, inner_b]),
        };

        let p = Property {
            name: "my_struct".into(),
            type_field: "struct MyStruct".into(),
            components: Some(vec![inner_foo, inner_bar]),
        };

        let params = vec![p];
        println!("params: {:?}\n", params);
        let selector = abi.build_fn_selector("my_func", &params).unwrap();

        assert_eq!(selector, "my_func(s(bool,s(u64,u32)))");
    }

    #[test]
    fn compiler_generated_abi_test() {
        let json_abi = r#"
        [
            {
                "inputs": [
                    {
                        "components": null,
                        "name": "gas",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color",
                        "type": "b256"
                    },
                    {
                        "components": null,
                        "name": "value",
                        "type": "u64"
                    }
                ],
                "name": "foo",
                "outputs": [
                    {
                        "components": null,
                        "name": "",
                        "type": "u64"
                    }
                ],
                "type": "function"
            },
            {
                "inputs": [
                    {
                        "components": null,
                        "name": "gas",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "amount",
                        "type": "u64"
                    },
                    {
                        "components": null,
                        "name": "color",
                        "type": "b256"
                    },
                    {
                        "components": [
                            {
                                "components": null,
                                "name": "a",
                                "type": "bool"
                            },
                            {
                                "components": null,
                                "name": "b",
                                "type": "u64"
                            }
                        ],
                        "name": "value",
                        "type": "struct TestStruct"
                    }
                ],
                "name": "boo",
                "outputs": [
                    {
                        "components": [
                            {
                                "components": null,
                                "name": "a",
                                "type": "bool"
                            },
                            {
                                "components": null,
                                "name": "b",
                                "type": "u64"
                            }
                        ],
                        "name": "",
                        "type": "struct TestStruct"
                    }
                ],
                "type": "function"
            }
        ]
                "#;

        let gas = "1000000".to_string();
        let coins = "0".to_string();
        let color = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let s = "(true, 42)".to_string();

        let values: Vec<String> = vec![gas, coins, color, s];

        let mut abi = ABIParser::new();

        let function_name = "boo";

        let encoded = abi
            .encode_with_function_selector(json_abi, function_name, &values)
            .unwrap();
        println!("encoded: {:?}\n", encoded);

        let expected_encode = "0000000087f27a3900000000000f4240000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000002a";
        assert_eq!(encoded, expected_encode);
    }
}
