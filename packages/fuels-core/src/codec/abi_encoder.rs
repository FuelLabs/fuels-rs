use fuel_types::bytes::padded_len_usize;
use itertools::Itertools;

use crate::{
    constants::WORD_SIZE,
    types::{
        errors::Result,
        pad_string, pad_u16, pad_u32, pad_u8,
        unresolved_bytes::{Data, UnresolvedBytes},
        EnumSelector, StringToken, Token, U256,
    },
};

pub struct ABIEncoder;

impl ABIEncoder {
    /// Encodes `Token`s in `args` following the ABI specs defined
    /// [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md)
    pub fn encode(args: &[Token]) -> Result<UnresolvedBytes> {
        let data = Self::encode_tokens(args)?;

        Ok(UnresolvedBytes::new(data))
    }

    fn encode_tokens(tokens: &[Token]) -> Result<Vec<Data>> {
        tokens
            .iter()
            .map(Self::encode_token)
            .flatten_ok()
            .collect::<Result<Vec<_>>>()
    }

    fn encode_token(arg: &Token) -> Result<Vec<Data>> {
        let encoded_token = match arg {
            Token::U8(arg_u8) => vec![Self::encode_u8(*arg_u8)],
            Token::U16(arg_u16) => vec![Self::encode_u16(*arg_u16)],
            Token::U32(arg_u32) => vec![Self::encode_u32(*arg_u32)],
            Token::U64(arg_u64) => vec![Self::encode_u64(*arg_u64)],
            Token::U128(arg_u128) => vec![Self::encode_u128(*arg_u128)],
            Token::U256(arg_u256) => vec![Self::encode_u256(*arg_u256)],
            Token::Bool(arg_bool) => vec![Self::encode_bool(*arg_bool)],
            Token::B256(arg_bits256) => vec![Self::encode_b256(arg_bits256)],
            Token::Array(arg_array) => Self::encode_array(arg_array)?,
            Token::Vector(data) => Self::encode_vector(data)?,
            Token::StringSlice(arg_string) => Self::encode_string_slice(arg_string)?,
            Token::StringArray(arg_string) => vec![Self::encode_string_array(arg_string)?],
            Token::Struct(arg_struct) => Self::encode_struct(arg_struct)?,
            Token::Enum(arg_enum) => Self::encode_enum(arg_enum)?,
            Token::Tuple(arg_tuple) => Self::encode_tuple(arg_tuple)?,
            Token::Unit => vec![Self::encode_unit()],
            Token::RawSlice(data) => Self::encode_raw_slice(data)?,
            Token::Bytes(data) => Self::encode_bytes(data.to_vec())?,
        };

        Ok(encoded_token)
    }

    fn encode_unit() -> Data {
        Data::Inline(vec![0; WORD_SIZE])
    }

    fn encode_tuple(arg_tuple: &[Token]) -> Result<Vec<Data>> {
        Self::encode_tokens(arg_tuple)
    }

    fn encode_struct(subcomponents: &[Token]) -> Result<Vec<Data>> {
        Self::encode_tokens(subcomponents)
    }

    fn encode_array(arg_array: &[Token]) -> Result<Vec<Data>> {
        Self::encode_tokens(arg_array)
    }

    fn encode_b256(arg_bits256: &[u8; 32]) -> Data {
        Data::Inline(arg_bits256.to_vec())
    }

    fn encode_bool(arg_bool: bool) -> Data {
        Data::Inline(pad_u8(u8::from(arg_bool)).to_vec())
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

    fn encode_u8(arg_u8: u8) -> Data {
        Data::Inline(pad_u8(arg_u8).to_vec())
    }

    fn encode_enum(selector: &EnumSelector) -> Result<Vec<Data>> {
        let (discriminant, token_within_enum, variants) = selector;

        let mut encoded_enum = vec![Self::encode_discriminant(*discriminant)];

        // Enums that contain only Units as variants have only their discriminant encoded.
        if !variants.only_units_inside() {
            let variant_param_type = variants.param_type_of_variant(*discriminant)?;
            let padding_amount = variants.compute_padding_amount(variant_param_type);

            encoded_enum.push(Data::Inline(vec![0; padding_amount]));

            let token_data = Self::encode_token(token_within_enum)?;
            encoded_enum.extend(token_data);
        }

        Ok(encoded_enum)
    }

    fn encode_discriminant(discriminant: u8) -> Data {
        Self::encode_u8(discriminant)
    }

    fn encode_vector(data: &[Token]) -> Result<Vec<Data>> {
        let encoded_data = Self::encode_tokens(data)?;
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

    fn encode_raw_slice(data: &[u64]) -> Result<Vec<Data>> {
        let encoded_data = data
            .iter()
            .map(|&word| Self::encode_u64(word))
            .collect::<Vec<_>>();

        let num_bytes = data.len() * WORD_SIZE;

        let len = Self::encode_u64(num_bytes as u64);
        Ok(vec![Data::Dynamic(encoded_data), len])
    }

    fn encode_string_slice(arg_string: &StringToken) -> Result<Vec<Data>> {
        let encoded_data = Data::Inline(arg_string.get_encodable_str()?.as_bytes().to_vec());

        let num_bytes = arg_string.get_encodable_str()?.len();
        let len = Self::encode_u64(num_bytes as u64);
        Ok(vec![Data::Dynamic(vec![encoded_data]), len])
    }

    fn encode_string_array(arg_string: &StringToken) -> Result<Data> {
        Ok(Data::Inline(pad_string(arg_string.get_encodable_str()?)))
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

#[cfg(test)]
mod tests {
    use std::slice;

    use itertools::chain;
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        codec::first_four_bytes_of_sha256_hash,
        types::{enum_variants::EnumVariants, param_types::ParamType},
    };

    const VEC_METADATA_SIZE: usize = 3 * WORD_SIZE;
    const DISCRIMINANT_SIZE: usize = WORD_SIZE;

    #[test]
    fn encode_function_signature() {
        let fn_signature = "entry_one(u64)";

        let result = first_four_bytes_of_sha256_hash(fn_signature);

        println!("Encoded function selector for ({fn_signature}): {result:#0x?}");

        assert_eq!(result, [0x0, 0x0, 0x0, 0x0, 0x0c, 0x36, 0xcb, 0x9c]);
    }

    #[test]
    fn encode_function_with_u32_type() -> Result<()> {
        // @todo eventually we must update the json abi examples in here.
        // They're in the old format.
        //
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"u32"}],
        //         "name":"entry_one",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "entry_one(u32)";
        let arg = Token::U32(u32::MAX);

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xb7, 0x9e, 0xf7, 0x43];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_u32_type_multiple_args() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"first","type":"u32"},{"name":"second","type":"u32"}],
        //         "name":"takes_two",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_two(u32,u32)";
        let first = Token::U32(u32::MAX);
        let second = Token::U32(u32::MAX);

        let args: Vec<Token> = vec![first, second];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff,
        ];

        let expected_fn_selector = [0x0, 0x0, 0x0, 0x0, 0xa7, 0x07, 0xb0, 0x8e];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);
        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_fn_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_u64_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"u64"}],
        //         "name":"entry_one",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "entry_one(u64)";
        let arg = Token::U64(u64::MAX);

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x0c, 0x36, 0xcb, 0x9c];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_bool_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"bool"}],
        //         "name":"bool_check",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "bool_check(bool)";
        let arg = Token::Bool(true);

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x66, 0x8f, 0xff, 0x58];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_two_different_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"first","type":"u32"},{"name":"second","type":"bool"}],
        //         "name":"takes_two_types",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_two_types(u32,bool)";
        let first = Token::U32(u32::MAX);
        let second = Token::Bool(true);

        let args: Vec<Token> = vec![first, second];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0xff, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xf5, 0x40, 0x73, 0x2b];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_bits256_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"b256"}],
        //         "name":"takes_bits256",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_bits256(b256)";

        let mut hasher = Sha256::new();
        hasher.update("test string".as_bytes());

        let arg = hasher.finalize();

        let arg = Token::B256(arg.into());

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, 0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44,
            0xe4, 0xcb, 0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, 0xa8, 0xf8, 0x27, 0x43,
            0xf3, 0x1e, 0x93, 0xb,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x01, 0x49, 0x42, 0x96];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_array_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"u8[3]"}],
        //         "name":"takes_integer_array",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_integer_array(u8[3])";

        // Keeping the construction of the arguments array separate for better readability.
        let first = Token::U8(1);
        let second = Token::U8(2);
        let third = Token::U8(3);

        let arg = vec![first, second, third];
        let arg_array = Token::Array(arg);

        let args: Vec<Token> = vec![arg_array];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x3,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x2c, 0x5a, 0x10, 0x2e];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_string_array_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"str[23]"}],
        //         "name":"takes_string",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_string(str[23])";

        let args: Vec<Token> = vec![Token::StringArray(StringToken::new(
            "This is a full sentence".into(),
            Some(23),
        ))];

        let expected_encoded_abi = [
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, 0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c,
            0x20, 0x73, 0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x00,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xd5, 0x6e, 0x76, 0x51];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_string_slice_type() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"str"}],
        //         "name":"takes_string",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_string(str)";

        let args: Vec<Token> = vec![Token::StringSlice(StringToken::new(
            "This is a full sentence".into(),
            None,
        ))];

        let expected_encoded_abi = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // str at data index 16
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17, // str of lenght 23
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, //
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, //
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, //
        ];

        let expected_function_selector = [0, 0, 0, 0, 239, 77, 222, 230];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_struct() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"MyStruct"}],
        //         "name":"takes_my_struct",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_my_struct(MyStruct)";

        // struct MyStruct {
        //     foo: u8,
        //     bar: bool,
        // }

        let foo = Token::U8(1);
        let bar = Token::Bool(true);

        // Create the custom struct token using the array of tuples above
        let arg = Token::Struct(vec![foo, bar]);

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xa8, 0x1e, 0x8d, 0xd7];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_function_with_enum() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"MyEnum"}],
        //         "name":"takes_my_enum",
        //         "outputs": []
        //     }
        // ]
        // "#;

        let fn_signature = "takes_my_enum(MyEnum)";

        // enum MyEnum {
        //     x: u32,
        //     y: bool,
        // }
        let types = vec![ParamType::U32, ParamType::Bool];
        let params = EnumVariants::new(types)?;

        // An `EnumSelector` indicating that we've chosen the first Enum variant,
        // whose value is 42 of the type ParamType::U32 and that the Enum could
        // have held any of the other types present in `params`.

        let enum_selector = Box::new((0, Token::U32(42), params));

        let arg = Token::Enum(enum_selector);

        let args: Vec<Token> = vec![arg];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2a,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x35, 0x5c, 0xa6, 0xfa];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    // The encoding follows the ABI specs defined  [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md)
    #[test]
    fn enums_are_sized_to_fit_the_biggest_variant() -> Result<()> {
        // Our enum has two variants: B256, and U64. So the enum will set aside
        // 256b of space or 4 WORDS because that is the space needed to fit the
        // largest variant(B256).
        let types = vec![ParamType::B256, ParamType::U64];
        let enum_variants = EnumVariants::new(types)?;
        let enum_selector = Box::new((1, Token::U64(42), enum_variants));

        let encoded = ABIEncoder::encode(slice::from_ref(&Token::Enum(enum_selector)))?.resolve(0);

        let enum_discriminant_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1];
        let u64_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2a];
        let enum_padding = vec![0x0; 24];

        // notice the ordering, first the discriminant, then the necessary
        // padding and then the value itself.
        let expected: Vec<u8> = [enum_discriminant_enc, enum_padding, u64_enc]
            .into_iter()
            .flatten()
            .collect();

        assert_eq!(hex::encode(expected), hex::encode(encoded));
        Ok(())
    }

    #[test]
    fn encoding_enums_with_deeply_nested_types() -> Result<()> {
        /*
        enum DeeperEnum {
            v1: bool,
            v2: str[10]
        }
         */
        let types = vec![ParamType::Bool, ParamType::String(10)];
        let deeper_enum_variants = EnumVariants::new(types)?;
        let deeper_enum_token = Token::StringArray(StringToken::new("0123456789".into(), Some(10)));

        let str_enc = vec![
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0,
        ];
        let deeper_enum_discriminant_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1];

        /*
        struct StructA {
            some_enum: DeeperEnum
            some_number: u32
        }
         */

        let fields = vec![
            ParamType::Enum {
                variants: deeper_enum_variants.clone(),
                generics: vec![],
            },
            ParamType::Bool,
        ];
        let struct_a_type = ParamType::Struct {
            fields,
            generics: vec![],
        };

        let struct_a_token = Token::Struct(vec![
            Token::Enum(Box::new((1, deeper_enum_token, deeper_enum_variants))),
            Token::U32(11332),
        ]);
        let some_number_enc = vec![0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2c, 0x44];

        /*
         enum TopLevelEnum {
            v1: StructA,
            v2: bool,
            v3: u64
        }
        */

        let types = vec![struct_a_type, ParamType::Bool, ParamType::U64];
        let top_level_enum_variants = EnumVariants::new(types)?;
        let top_level_enum_token =
            Token::Enum(Box::new((0, struct_a_token, top_level_enum_variants)));
        let top_lvl_discriminant_enc = vec![0x0; 8];

        let encoded = ABIEncoder::encode(slice::from_ref(&top_level_enum_token))?.resolve(0);

        let correct_encoding: Vec<u8> = [
            top_lvl_discriminant_enc,
            deeper_enum_discriminant_enc,
            str_enc,
            some_number_enc,
        ]
        .into_iter()
        .flatten()
        .collect();

        assert_eq!(hex::encode(correct_encoding), hex::encode(encoded));
        Ok(())
    }

    #[test]
    fn encode_function_with_nested_structs() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type":"function",
        //         "inputs": [{"name":"arg","type":"Foo"}],
        //         "name":"takes_my_nested_struct",
        //         "outputs": []
        //     }
        // ]
        // "#;

        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        let fn_signature = "takes_my_nested_struct(Foo)";

        let args: Vec<Token> = vec![Token::Struct(vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ])];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0,
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2,
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xea, 0x0a, 0xfd, 0x23];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        println!("Encoded ABI for ({fn_signature}): {encoded:#0x?}");

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn encode_comprehensive_function() -> Result<()> {
        // let json_abi =
        // r#"
        // [
        //     {
        //         "type": "contract",
        //         "inputs": [
        //         {
        //             "name": "arg",
        //             "type": "Foo"
        //         },
        //         {
        //             "name": "arg2",
        //             "type": "u8[2]"
        //         },
        //         {
        //             "name": "arg3",
        //             "type": "b256"
        //         },
        //         {
        //             "name": "arg",
        //             "type": "str[23]"
        //         }
        //         ],
        //         "name": "long_function",
        //         "outputs": []
        //     }
        // ]
        // "#;

        // struct Foo {
        //     x: u16,
        //     y: Bar,
        // }
        //
        // struct Bar {
        //     a: bool,
        //     b: u8[2],
        // }

        let fn_signature = "long_function(Foo,u8[2],b256,str[23])";

        let foo = Token::Struct(vec![
            Token::U16(10),
            Token::Struct(vec![
                Token::Bool(true),
                Token::Array(vec![Token::U8(1), Token::U8(2)]),
            ]),
        ]);

        let u8_arr = Token::Array(vec![Token::U8(1), Token::U8(2)]);

        let mut hasher = Sha256::new();
        hasher.update("test string".as_bytes());

        let b256 = Token::B256(hasher.finalize().into());

        let s = Token::StringArray(StringToken::new("This is a full sentence".into(), Some(23)));

        let args: Vec<Token> = vec![foo, u8_arr, b256, s];

        let expected_encoded_abi = [
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xa, // foo.x == 10u16
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // foo.y.a == true
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // foo.b.0 == 1u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, // foo.b.1 == 2u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, // u8[2].0 == 1u8
            0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, // u8[2].0 == 2u8
            0xd5, 0x57, 0x9c, 0x46, 0xdf, 0xcc, 0x7f, 0x18, // b256
            0x20, 0x70, 0x13, 0xe6, 0x5b, 0x44, 0xe4, 0xcb, // b256
            0x4e, 0x2c, 0x22, 0x98, 0xf4, 0xac, 0x45, 0x7b, // b256
            0xa8, 0xf8, 0x27, 0x43, 0xf3, 0x1e, 0x93, 0xb, // b256
            0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73, 0x20, // str[23]
            0x61, 0x20, 0x66, 0x75, 0x6c, 0x6c, 0x20, 0x73, // str[23]
            0x65, 0x6e, 0x74, 0x65, 0x6e, 0x63, 0x65, 0x0, // str[23]
        ];

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x10, 0x93, 0xb2, 0x12];

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let encoded = ABIEncoder::encode(&args)?.resolve(0);

        assert_eq!(hex::encode(expected_encoded_abi), hex::encode(encoded));
        assert_eq!(encoded_function_selector, expected_function_selector);
        Ok(())
    }

    #[test]
    fn enums_with_only_unit_variants_are_encoded_in_one_word() -> Result<()> {
        let expected = [0, 0, 0, 0, 0, 0, 0, 1];

        let types = vec![ParamType::Unit, ParamType::Unit];
        let enum_selector = Box::new((1, Token::Unit, EnumVariants::new(types)?));

        let actual = ABIEncoder::encode(&[Token::Enum(enum_selector)])?.resolve(0);

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn units_in_composite_types_are_encoded_in_one_word() -> Result<()> {
        let expected = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5];

        let actual =
            ABIEncoder::encode(&[Token::Struct(vec![Token::Unit, Token::U32(5)])])?.resolve(0);

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn enums_with_units_are_correctly_padded() -> Result<()> {
        let discriminant = vec![0, 0, 0, 0, 0, 0, 0, 1];
        let padding = vec![0; 32];
        let expected: Vec<u8> = [discriminant, padding].into_iter().flatten().collect();

        let types = vec![ParamType::B256, ParamType::Unit];
        let enum_selector = Box::new((1, Token::Unit, EnumVariants::new(types)?));

        let actual = ABIEncoder::encode(&[Token::Enum(enum_selector)])?.resolve(0);

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn vector_has_ptr_cap_len_and_then_data() -> Result<()> {
        // arrange
        let offset: u8 = 150;
        let token = Token::Vector(vec![Token::U64(5)]);

        // act
        let result = ABIEncoder::encode(&[token])?.resolve(offset as u64);

        // assert
        let ptr = [0, 0, 0, 0, 0, 0, 0, 3 * WORD_SIZE as u8 + offset];
        let cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let len = [0, 0, 0, 0, 0, 0, 0, 1];
        let data = [0, 0, 0, 0, 0, 0, 0, 5];

        let expected = chain!(ptr, cap, len, data).collect::<Vec<_>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn data_from_two_vectors_aggregated_at_the_end() -> Result<()> {
        // arrange
        let offset: u8 = 40;
        let vec_1 = Token::Vector(vec![Token::U64(5)]);
        let vec_2 = Token::Vector(vec![Token::U64(6)]);

        // act
        let result = ABIEncoder::encode(&[vec_1, vec_2])?.resolve(offset as u64);

        // assert
        let vec1_data_offset = 6 * WORD_SIZE as u8 + offset;
        let vec1_ptr = [0, 0, 0, 0, 0, 0, 0, vec1_data_offset];
        let vec1_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_len = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_data = [0, 0, 0, 0, 0, 0, 0, 5];

        let vec2_data_offset = vec1_data_offset + vec1_data.len() as u8;
        let vec2_ptr = [0, 0, 0, 0, 0, 0, 0, vec2_data_offset];
        let vec2_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec2_len = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec2_data = [0, 0, 0, 0, 0, 0, 0, 6];

        let expected = chain!(
            vec1_ptr, vec1_cap, vec1_len, vec2_ptr, vec2_cap, vec2_len, vec1_data, vec2_data,
        )
        .collect::<Vec<_>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn a_vec_in_an_enum() -> Result<()> {
        // arrange
        let offset = 40;
        let types = vec![ParamType::B256, ParamType::Vector(Box::new(ParamType::U64))];
        let variants = EnumVariants::new(types)?;
        let selector = (1, Token::Vector(vec![Token::U64(5)]), variants);
        let token = Token::Enum(Box::new(selector));

        // act
        let result = ABIEncoder::encode(&[token])?.resolve(offset as u64);

        // assert
        let discriminant = vec![0, 0, 0, 0, 0, 0, 0, 1];

        const PADDING: usize = std::mem::size_of::<[u8; 32]>() - VEC_METADATA_SIZE;

        let vec1_ptr = ((DISCRIMINANT_SIZE + PADDING + VEC_METADATA_SIZE + offset) as u64)
            .to_be_bytes()
            .to_vec();
        let vec1_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_len = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_data = [0, 0, 0, 0, 0, 0, 0, 5];

        let expected = chain!(
            discriminant,
            vec![0; PADDING],
            vec1_ptr,
            vec1_cap,
            vec1_len,
            vec1_data
        )
        .collect::<Vec<u8>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn an_enum_in_a_vec() -> Result<()> {
        // arrange
        let offset = 40;
        let types = vec![ParamType::B256, ParamType::U8];
        let variants = EnumVariants::new(types)?;
        let selector = (1, Token::U8(8), variants);
        let enum_token = Token::Enum(Box::new(selector));

        let vec_token = Token::Vector(vec![enum_token]);

        // act
        let result = ABIEncoder::encode(&[vec_token])?.resolve(offset as u64);

        // assert
        const PADDING: usize = std::mem::size_of::<[u8; 32]>() - WORD_SIZE;

        let vec1_ptr = ((VEC_METADATA_SIZE + offset) as u64).to_be_bytes().to_vec();
        let vec1_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_len = [0, 0, 0, 0, 0, 0, 0, 1];
        let discriminant = 1u64.to_be_bytes();
        let vec1_data = chain!(discriminant, [0; PADDING], 8u64.to_be_bytes()).collect::<Vec<_>>();

        let expected = chain!(vec1_ptr, vec1_cap, vec1_len, vec1_data).collect::<Vec<u8>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn a_vec_in_a_struct() -> Result<()> {
        // arrange
        let offset = 40;
        let token = Token::Struct(vec![Token::Vector(vec![Token::U64(5)]), Token::U8(9)]);

        // act
        let result = ABIEncoder::encode(&[token])?.resolve(offset as u64);

        // assert
        let vec1_ptr = ((VEC_METADATA_SIZE + WORD_SIZE + offset) as u64)
            .to_be_bytes()
            .to_vec();
        let vec1_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_len = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_data = [0, 0, 0, 0, 0, 0, 0, 5];

        let expected = chain!(
            vec1_ptr,
            vec1_cap,
            vec1_len,
            [0, 0, 0, 0, 0, 0, 0, 9],
            vec1_data
        )
        .collect::<Vec<u8>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn a_vec_in_a_vec() -> Result<()> {
        // arrange
        let offset = 40;
        let token = Token::Vector(vec![Token::Vector(vec![Token::U8(5), Token::U8(6)])]);

        // act
        let result = ABIEncoder::encode(&[token])?.resolve(offset as u64);

        // assert
        let vec1_data_offset = (VEC_METADATA_SIZE + offset) as u64;
        let vec1_ptr = vec1_data_offset.to_be_bytes().to_vec();
        let vec1_cap = [0, 0, 0, 0, 0, 0, 0, 1];
        let vec1_len = [0, 0, 0, 0, 0, 0, 0, 1];

        let vec2_ptr = (vec1_data_offset + VEC_METADATA_SIZE as u64)
            .to_be_bytes()
            .to_vec();
        let vec2_cap = [0, 0, 0, 0, 0, 0, 0, 2];
        let vec2_len = [0, 0, 0, 0, 0, 0, 0, 2];
        let vec2_data = [0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 6];

        let vec1_data = chain!(vec2_ptr, vec2_cap, vec2_len, vec2_data).collect::<Vec<_>>();

        let expected = chain!(vec1_ptr, vec1_cap, vec1_len, vec1_data).collect::<Vec<u8>>();

        assert_eq!(result, expected);

        Ok(())
    }

    #[test]
    fn encoding_bytes() -> Result<()> {
        // arrange
        let token = Token::Bytes(vec![1, 2, 3]);
        let offset = 40;

        // act
        let encoded_bytes = ABIEncoder::encode(&[token])?.resolve(offset);

        // assert
        let ptr = [0, 0, 0, 0, 0, 0, 0, 64];
        let cap = [0, 0, 0, 0, 0, 0, 0, 8];
        let len = [0, 0, 0, 0, 0, 0, 0, 3];
        let data = [1, 2, 3, 0, 0, 0, 0, 0];

        let expected_encoded_bytes = [ptr, cap, len, data].concat();

        assert_eq!(expected_encoded_bytes, encoded_bytes);

        Ok(())
    }

    #[test]
    fn encoding_raw_slices() -> Result<()> {
        // arrange
        let token = Token::RawSlice(vec![1, 2, 3]);
        let offset = 40;

        // act
        let encoded_bytes = ABIEncoder::encode(&[token])?.resolve(offset);

        // assert
        let ptr = vec![0, 0, 0, 0, 0, 0, 0, 56];
        let len = vec![0, 0, 0, 0, 0, 0, 0, 24];
        let data = [
            [0, 0, 0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 0, 0, 2],
            [0, 0, 0, 0, 0, 0, 0, 3],
        ]
        .concat();

        let expected_encoded_bytes = [ptr, len, data].concat();

        assert_eq!(expected_encoded_bytes, encoded_bytes);

        Ok(())
    }
}
