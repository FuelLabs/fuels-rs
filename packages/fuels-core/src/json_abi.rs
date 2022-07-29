use crate::tokenizer::Tokenizer;
use crate::utils::first_four_bytes_of_sha256_hash;
use crate::Token;
use crate::{abi_decoder::ABIDecoder, abi_encoder::ABIEncoder};
use fuels_types::function_selector::build_fn_selector;
use fuels_types::{errors::Error, param_types::ParamType, JsonABI, Property};
use itertools::Itertools;
use serde_json;
use std::str;

pub struct ABIParser {
    fn_selector: Option<Vec<u8>>,
}

impl Default for ABIParser {
    fn default() -> Self {
        Self::new()
    }
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
    /// use fuels_core::json_abi::ABIParser;
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

        let entry = entry.expect("No functions found");

        let fn_selector = build_fn_selector(fn_name, &entry.inputs)?;

        // Update the fn_selector field with the hash of the previously encoded function selector
        self.fn_selector = Some(first_four_bytes_of_sha256_hash(&fn_selector).to_vec());

        let params_and_values = entry
            .inputs
            .iter()
            .zip(values)
            .map(|(prop, val)| Ok((ParamType::try_from(prop)?, val.as_str())))
            .collect::<Result<Vec<_>, Error>>()?;

        let tokens = self.parse_tokens(&params_and_values)?;

        Ok(hex::encode(ABIEncoder::encode(&tokens)?))
    }

    /// Similar to `encode`, but includes the function selector in the
    /// final encoded string.
    ///
    /// # Examples
    /// ```
    /// use fuels_core::json_abi::ABIParser;
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

        let encoded_function_selector = hex::encode(fn_selector);

        Ok(format!("{}{}", encoded_function_selector, encoded_params))
    }

    /// Similar to `encode`, but it encodes only an array of strings containing
    /// [<type_1>, <param_1>, <type_2>, <param_2>, <type_n>, <param_n>]
    /// Without having to reference to a JSON specification of the ABI.
    pub fn encode_params(&self, params: &[String]) -> Result<String, Error> {
        let pairs: Vec<_> = params.chunks(2).collect_vec();

        let mut param_type_pairs: Vec<(ParamType, &str)> = vec![];

        for pair in pairs {
            let prop = Property {
                name: "".to_string(),
                type_field: pair[0].clone(),
                components: None,
            };
            let p = ParamType::try_from(&prop)?;

            let t: (ParamType, &str) = (p, &pair[1]);
            param_type_pairs.push(t);
        }

        let tokens = self.parse_tokens(&param_type_pairs)?;

        let encoded = ABIEncoder::encode(&tokens)?;

        Ok(hex::encode(encoded))
    }

    /// Helper function to turn a list of tuples(ParamType, &str) into
    /// a vector of Tokens ready to be encoded.
    /// Essentially a wrapper on `tokenize`.
    pub fn parse_tokens<'a>(&self, params: &'a [(ParamType, &str)]) -> Result<Vec<Token>, Error> {
        params
            .iter()
            .map(|&(ref param, value)| Tokenizer::tokenize(param, value.to_string()))
            .collect::<Result<_, _>>()
            .map_err(From::from)
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
            return Err(Error::InvalidData(format!(
                "couldn't find function name: {}",
                fn_name
            )));
        }

        let params_result: Result<Vec<_>, _> = entry
            .unwrap()
            .outputs
            .iter()
            .map(ParamType::try_from)
            .collect();

        match params_result {
            Ok(params) => Ok(ABIDecoder::decode(&params, value)?),
            Err(e) => Err(e),
        }
    }

    /// Similar to decode, but it decodes only an array types and the encoded data
    /// without having to reference to a JSON specification of the ABI.
    pub fn decode_params(&self, params: &[ParamType], data: &[u8]) -> Result<Vec<Token>, Error> {
        Ok(ABIDecoder::decode(params, data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::errors::Error;

    #[test]
    fn simple_encode_and_decode_no_selector() -> Result<(), Error> {
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

        let encoded = abi.encode(json_abi, function_name, &values)?;

        let expected_encode = "000000000000000a";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // false
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let expected_return = vec![Token::Bool(false)];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn simple_encode_and_decode() -> Result<(), Error> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "000000006355e6ee000000000000000a";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // false
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let expected_return = vec![Token::Bool(false)];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn b256_and_single_byte_encode_and_decode() -> Result<(), Box<dyn std::error::Error>> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "00000000e64019abd5579c46dfcc7f18207013e65b44e4cb4e2c2298f4ac457ba8f82743f31e930b0000000000000001";
        assert_eq!(encoded, expected_encode);

        let return_value =
            hex::decode("a441b15fe9a3cf56661190a0b93b9dec7d04127288cc87250967cf3b52894d11")?;

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let s: [u8; 32] = return_value.as_slice().try_into()?;

        let expected_return = vec![Token::B256(s)];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn array_encode_and_decode() -> Result<(), Error> {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"[u16; 3]"
                    }
                ],
                "name":"takes_array",
                "outputs":[
                    {
                        "name":"",
                        "type":"[u16; 2]"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["[1,2,3]".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_array";

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "00000000101cbeb5000000000000000100000000000000020000000000000003";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // 0
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // 1
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let expected_return = vec![Token::Array(vec![Token::U16(0), Token::U16(1)])];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn nested_array_encode_and_decode() -> Result<(), Error> {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"[u16; 3]"
                    }
                ],
                "name":"takes_nested_array",
                "outputs":[
                    {
                        "name":"",
                        "type":"[u16; 2]"
                    }
                ]
            }
        ]
        "#;

        let values: Vec<String> = vec!["[[1,2],[3],[4]]".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_nested_array";

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode =
            "00000000e6a030f00000000000000001000000000000000200000000000000030000000000000004";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // 0
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // 1
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let expected_return = vec![Token::Array(vec![Token::U16(0), Token::U16(1)])];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn string_encode_and_decode() -> Result<(), Error> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "00000000d56e76515468697320697320612066756c6c2073656e74656e636500";
        assert_eq!(encoded, expected_encode);

        let return_value = [
            0x4f, 0x4b, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, // "OK" encoded in utf8
        ];

        let decoded_return = abi.decode(json_abi, function_name, &return_value)?;

        let expected_return = vec![Token::String("OK".into())];

        assert_eq!(decoded_return, expected_return);
        Ok(())
    }

    #[test]
    fn struct_encode_and_decode() -> Result<(), Error> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "00000000cb0b2f05000000000000002a0000000000000001";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn struct_and_primitive_encode_and_decode() -> Result<(), Error> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "000000005c445838000000000000002a0000000000000001000000000000000a";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn nested_struct_encode_and_decode() -> Result<(), Error> {
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
                                        "type": "[u8; 2]"
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode =
            "00000000b1fbe7e3000000000000000a000000000000000100000000000000010000000000000002";
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
                                        "type": "[u8; 2]"
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode =
            "00000000e748f310000000000000000100000000000000010000000000000002000000000000000a";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn tuple_encode_and_decode() -> Result<(), Error> {
        let json_abi = r#"
        [
            {
                "type":"contract",
                "inputs": [
                  {
                    "name": "input",
                    "type": "(u64, bool)",
                    "components": [
                      {
                        "name": "__tuple_element",
                        "type": "u64",
                        "components": null
                      },
                      {
                        "name": "__tuple_element",
                        "type": "bool",
                        "components": null
                      }
                    ]
                  }
                ],
                "name":"takes_tuple",
                "outputs":[]
            }
        ]
        "#;

        let values: Vec<String> = vec!["(42, true)".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_tuple";

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "000000001cc7bb2c000000000000002a0000000000000001";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn nested_tuple_encode_and_decode() -> Result<(), Error> {
        let json_abi = r#"
        [
          {
            "type": "function",
            "inputs": [
              {
                "name": "input",
                "type": "((u64, bool), struct Person, enum State)",
                "components": [
                  {
                    "name": "__tuple_element",
                    "type": "(u64, bool)",
                    "components": [
                      {
                        "name": "__tuple_element",
                        "type": "u64",
                        "components": null
                      },
                      {
                        "name": "__tuple_element",
                        "type": "bool",
                        "components": null
                      }
                    ]
                  },
                  {
                    "name": "__tuple_element",
                    "type": "struct Person",
                    "components": [
                      {
                        "name": "name",
                        "type": "str[4]",
                        "components": null
                      }
                    ]
                  },
                  {
                    "name": "__tuple_element",
                    "type": "enum State",
                    "components": [
                      {
                        "name": "A",
                        "type": "()",
                        "components": []
                      },
                      {
                        "name": "B",
                        "type": "()",
                        "components": []
                      },
                      {
                        "name": "C",
                        "type": "()",
                        "components": []
                      }
                    ]
                  }
                ]
              }
            ],
            "name": "takes_nested_tuple",
            "outputs":[]
          }
        ]
        "#;

        let values: Vec<String> = vec!["((42, true), (John), (1, 0))".to_string()];

        let mut abi = ABIParser::new();

        let function_name = "takes_nested_tuple";

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        println!("Function: {}", hex::encode(abi.fn_selector.unwrap()));
        let expected_encode =
            "00000000ebb8d011000000000000002a00000000000000014a6f686e000000000000000000000001";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn enum_encode_and_decode() -> Result<(), Error> {
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

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "0000000021b2784f0000000000000000000000000000002a";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }

    #[test]
    fn fn_selector_single_primitive() -> Result<(), Error> {
        let p = Property {
            name: "foo".into(),
            type_field: "u64".into(),
            components: None,
        };
        let params = vec![p];
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(u64)");
        Ok(())
    }

    #[test]
    fn fn_selector_multiple_primitives() -> Result<(), Error> {
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
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(u64,bool)");
        Ok(())
    }

    #[test]
    fn fn_selector_custom_type() -> Result<(), Error> {
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

        let p_struct = Property {
            name: "my_struct".into(),
            type_field: "struct MyStruct".into(),
            components: Some(vec![inner_foo.clone(), inner_bar.clone()]),
        };

        let params = vec![p_struct];
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(s(bool,u64))");

        let p_enum = Property {
            name: "my_enum".into(),
            type_field: "enum MyEnum".into(),
            components: Some(vec![inner_foo, inner_bar]),
        };
        let params = vec![p_enum];
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(e(bool,u64))");
        Ok(())
    }

    #[test]
    fn fn_selector_nested_struct() -> Result<(), Error> {
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
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(s(bool,s(u64,u32)))");
        Ok(())
    }

    #[test]
    fn fn_selector_nested_enum() -> Result<(), Error> {
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
            type_field: "enum InnerEnum".into(),
            components: Some(vec![inner_a, inner_b]),
        };

        let p = Property {
            name: "my_enum".into(),
            type_field: "enum MyEnum".into(),
            components: Some(vec![inner_foo, inner_bar]),
        };

        let params = vec![p];
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(e(bool,e(u64,u32)))");
        Ok(())
    }

    #[test]
    fn fn_selector_nested_custom_types() -> Result<(), Error> {
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

        let mut inner_custom = Property {
            name: "bar".into(),
            type_field: "enum InnerEnum".into(),
            components: Some(vec![inner_a, inner_b]),
        };

        let p = Property {
            name: "my_struct".into(),
            type_field: "struct MyStruct".into(),
            components: Some(vec![inner_foo.clone(), inner_custom.clone()]),
        };

        let params = vec![p];
        let selector = build_fn_selector("my_func", &params)?;

        assert_eq!(selector, "my_func(s(bool,e(u64,u32)))");

        inner_custom.type_field = "struct InnerStruct".to_string();
        let p = Property {
            name: "my_enum".into(),
            type_field: "enum MyEnum".into(),
            components: Some(vec![inner_foo, inner_custom]),
        };
        let params = vec![p];
        let selector = build_fn_selector("my_func", &params)?;
        assert_eq!(selector, "my_func(e(bool,s(u64,u32)))");
        Ok(())
    }

    #[test]
    fn compiler_generated_abi_test() -> Result<(), Error> {
        let json_abi = r#"
        [
            {
                "inputs": [
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

        let s = "(true, 42)".to_string();

        let values: Vec<String> = vec![s];

        let mut abi = ABIParser::new();

        let function_name = "boo";

        let encoded = abi.encode_with_function_selector(json_abi, function_name, &values)?;

        let expected_encode = "00000000e33a11ce0000000000000001000000000000002a";
        assert_eq!(encoded, expected_encode);
        Ok(())
    }
}
