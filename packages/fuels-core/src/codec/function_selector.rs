#[cfg(feature = "legacy_encoding")]
use sha2::{Digest, Sha256};

#[cfg(feature = "legacy_encoding")]
use crate::types::param_types::NamedParamType;
use crate::types::param_types::ParamType;
#[cfg(feature = "legacy_encoding")]
use crate::types::ByteArray;

#[cfg(feature = "legacy_encoding")]
/// Given a function name and its inputs  will return a ByteArray representing
/// the function selector as specified in the Fuel specs.
pub fn resolve_fn_selector(name: &str, inputs: &[ParamType]) -> ByteArray {
    let fn_signature = resolve_fn_signature(name, inputs);

    first_four_bytes_of_sha256_hash(&fn_signature)
}

#[cfg(not(feature = "legacy_encoding"))]
//TODO: remove `_inputs` once the new encoding stabilizes
//https://github.com/FuelLabs/fuels-rs/issues/1318
pub fn resolve_fn_selector(name: &str, _inputs: &[ParamType]) -> Vec<u8> {
    let bytes = name.as_bytes().to_vec();
    let len = bytes.len() as u64;

    [len.to_be_bytes().to_vec(), bytes].concat()
}

#[cfg(feature = "legacy_encoding")]
fn resolve_fn_signature(name: &str, inputs: &[ParamType]) -> String {
    let fn_args = resolve_args(inputs);

    format!("{name}({fn_args})")
}

#[cfg(feature = "legacy_encoding")]
fn resolve_args(args: &[ParamType]) -> String {
    args.iter().map(resolve_arg).collect::<Vec<_>>().join(",")
}

#[cfg(feature = "legacy_encoding")]
fn resolve_named_args(args: &[NamedParamType]) -> String {
    args.iter()
        .map(|(_, param_type)| resolve_arg(param_type))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(feature = "legacy_encoding")]
fn resolve_arg(arg: &ParamType) -> String {
    match &arg {
        ParamType::U8 => "u8".to_owned(),
        ParamType::U16 => "u16".to_owned(),
        ParamType::U32 => "u32".to_owned(),
        ParamType::U64 => "u64".to_owned(),
        ParamType::U128 => "s(u64,u64)".to_owned(),
        ParamType::U256 => "u256".to_owned(),
        ParamType::Bool => "bool".to_owned(),
        ParamType::B256 => "b256".to_owned(),
        ParamType::Unit => "()".to_owned(),
        ParamType::StringSlice => "str".to_owned(),
        ParamType::StringArray(len) => {
            format!("str[{len}]")
        }
        ParamType::Array(internal_type, len) => {
            let inner = resolve_arg(internal_type);
            format!("a[{inner};{len}]")
        }
        ParamType::Struct {
            fields, generics, ..
        } => {
            let gen_params = resolve_args(generics);
            let field_params = resolve_named_args(fields);
            let gen_params = if !gen_params.is_empty() {
                format!("<{gen_params}>")
            } else {
                gen_params
            };
            format!("s{gen_params}({field_params})")
        }
        ParamType::Enum {
            enum_variants,
            generics,
            ..
        } => {
            let gen_params = resolve_args(generics);
            let field_params = resolve_named_args(enum_variants.variants());
            let gen_params = if !gen_params.is_empty() {
                format!("<{gen_params}>")
            } else {
                gen_params
            };
            format!("e{gen_params}({field_params})")
        }
        ParamType::Tuple(inner) => {
            let inner = resolve_args(inner);
            format!("({inner})")
        }
        ParamType::Vector(el_type) => {
            let inner = resolve_arg(el_type);
            format!("s<{inner}>(s<{inner}>(rawptr,u64),u64)")
        }
        ParamType::RawSlice => "rawslice".to_string(),
        ParamType::Bytes => "s(s(rawptr,u64),u64)".to_string(),
        ParamType::String => "s(s(s(rawptr,u64),u64))".to_string(),
    }
}

#[cfg(feature = "legacy_encoding")]
/// Hashes an encoded function selector using SHA256 and returns the first 4 bytes.
/// The function selector has to have been already encoded following the ABI specs defined
/// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
pub(crate) fn first_four_bytes_of_sha256_hash(string: &str) -> ByteArray {
    let string_as_bytes = string.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(string_as_bytes);
    let result = hasher.finalize();
    let mut output = ByteArray::default();
    output[4..].copy_from_slice(&result[..4]);
    output
}

#[macro_export]
macro_rules! fn_selector {
    ( $fn_name: ident ( $($fn_arg: ty),* )  ) => {
         ::fuels::core::codec::resolve_fn_selector(
                 stringify!($fn_name),
                 &[$( <$fn_arg as ::fuels::core::traits::Parameterize>::param_type() ),*]
             )
             .to_vec()
    }
}

pub use fn_selector;

/// This uses the default `EncoderConfig` configuration.
#[macro_export]
macro_rules! calldata {
    ( $($arg: expr),* ) => {
        ::fuels::core::codec::ABIEncoder::default().encode(&[$(::fuels::core::traits::Tokenizable::into_token($arg)),*])
            .map(|ub| ub.resolve(0))
    }
}

pub use calldata;

#[cfg(feature = "legacy_encoding")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{to_named, types::param_types::EnumVariants};

    #[test]
    fn handles_primitive_types() {
        let check_selector_for_type = |primitive_type: ParamType, expected_selector: &str| {
            let selector = resolve_fn_signature("some_fun", &[primitive_type]);

            assert_eq!(selector, format!("some_fun({expected_selector})"));
        };

        for (param_type, expected_signature) in [
            (ParamType::U8, "u8"),
            (ParamType::U16, "u16"),
            (ParamType::U32, "u32"),
            (ParamType::U64, "u64"),
            (ParamType::Bool, "bool"),
            (ParamType::B256, "b256"),
            (ParamType::Unit, "()"),
            (ParamType::StringArray(15), "str[15]"),
            (ParamType::StringSlice, "str"),
        ] {
            check_selector_for_type(param_type, expected_signature);
        }
    }

    #[test]
    fn handles_std_strings() {
        let inputs = [ParamType::String];

        let signature = resolve_fn_signature("some_fn", &inputs);

        assert_eq!(signature, "some_fn(s(s(s(rawptr,u64),u64)))");
    }

    #[test]
    fn handles_arrays() {
        let inputs = [ParamType::Array(Box::new(ParamType::U8), 1)];

        let signature = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(signature, format!("some_fun(a[u8;1])"));
    }

    #[test]
    fn handles_tuples() {
        let inputs = [ParamType::Tuple(vec![ParamType::U8, ParamType::U8])];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, format!("some_fun((u8,u8))"));
    }

    #[test]
    fn handles_structs() {
        let fields = to_named(&[ParamType::U64, ParamType::U32]);
        let generics = vec![ParamType::U32];
        let inputs = [ParamType::Struct {
            name: "".to_string(),
            fields,
            generics,
        }];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, format!("some_fun(s<u32>(u64,u32))"));
    }

    #[test]
    fn handles_vectors() {
        let inputs = [ParamType::Vector(Box::new(ParamType::U32))];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, "some_fun(s<u32>(s<u32>(rawptr,u64),u64))")
    }

    #[test]
    fn handles_bytes() {
        let inputs = [ParamType::Bytes];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, "some_fun(s(s(rawptr,u64),u64))")
    }

    #[test]
    fn handles_enums() {
        let types = to_named(&[ParamType::U64, ParamType::U32]);
        let enum_variants = EnumVariants::new(types).unwrap();
        let generics = vec![ParamType::U32];
        let inputs = [ParamType::Enum {
            name: "".to_string(),
            enum_variants,
            generics,
        }];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, format!("some_fun(e<u32>(u64,u32))"));
    }

    #[test]
    fn ultimate_test() {
        let fields = to_named(&[ParamType::Struct {
            name: "".to_string(),

            fields: to_named(&[ParamType::StringArray(2)]),
            generics: vec![ParamType::StringArray(2)],
        }]);
        let struct_a = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![ParamType::StringArray(2)],
        };

        let fields = to_named(&[ParamType::Array(Box::new(struct_a.clone()), 2)]);
        let struct_b = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![struct_a],
        };

        let fields = to_named(&[ParamType::Tuple(vec![struct_b.clone(), struct_b.clone()])]);
        let struct_c = ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![struct_b],
        };

        let types = to_named(&[ParamType::U64, struct_c.clone()]);
        let fields = to_named(&[
            ParamType::Tuple(vec![
                ParamType::Array(Box::new(ParamType::B256), 2),
                ParamType::StringArray(2),
            ]),
            ParamType::Tuple(vec![
                ParamType::Array(
                    Box::new(ParamType::Enum {
                        name: "".to_string(),
                        enum_variants: EnumVariants::new(types).unwrap(),
                        generics: vec![struct_c],
                    }),
                    1,
                ),
                ParamType::U32,
            ]),
        ]);

        let inputs = [ParamType::Struct {
            name: "".to_string(),
            fields,
            generics: vec![ParamType::StringArray(2), ParamType::B256],
        }];

        let selector = resolve_fn_signature("complex_test", &inputs);

        assert_eq!(selector, "complex_test(s<str[2],b256>((a[b256;2],str[2]),(a[e<s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])))>(u64,s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]))));1],u32)))");
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_signature() {
        let fn_signature = "entry_one(u64)";

        let result = first_four_bytes_of_sha256_hash(fn_signature);

        assert_eq!(result, [0x0, 0x0, 0x0, 0x0, 0x0c, 0x36, 0xcb, 0x9c]);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_u32_type() {
        let fn_signature = "entry_one(u32)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xb7, 0x9e, 0xf7, 0x43];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_u32_type_multiple_args() {
        let fn_signature = "takes_two(u32,u32)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_fn_selector = [0x0, 0x0, 0x0, 0x0, 0xa7, 0x07, 0xb0, 0x8e];

        assert_eq!(encoded_function_selector, expected_fn_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_u64_type() {
        let fn_signature = "entry_one(u64)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x0c, 0x36, 0xcb, 0x9c];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_bool_type() {
        let fn_signature = "bool_check(bool)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x66, 0x8f, 0xff, 0x58];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_two_different_type() {
        let fn_signature = "takes_two_types(u32,bool)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xf5, 0x40, 0x73, 0x2b];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_bits256_type() {
        let fn_signature = "takes_bits256(b256)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x01, 0x49, 0x42, 0x96];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_array_type() {
        let fn_signature = "takes_integer_array(u8[3])";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x2c, 0x5a, 0x10, 0x2e];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_string_array_type() {
        let fn_signature = "takes_string(str[23])";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xd5, 0x6e, 0x76, 0x51];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_string_slice_type() {
        let fn_signature = "takes_string(str)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0, 0, 0, 0, 239, 77, 222, 230];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_struct() {
        let fn_signature = "takes_my_struct(MyStruct)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0xa8, 0x1e, 0x8d, 0xd7];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_function_with_enum() {
        let fn_signature = "takes_my_enum(MyEnum)";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x35, 0x5c, 0xa6, 0xfa];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }

    #[test]
    #[cfg(feature = "legacy_encoding")]
    fn encode_comprehensive_function() {
        let fn_signature = "long_function(Foo,u8[2],b256,str[23])";

        let encoded_function_selector = first_four_bytes_of_sha256_hash(fn_signature);

        let expected_function_selector = [0x0, 0x0, 0x0, 0x0, 0x10, 0x93, 0xb2, 0x12];

        assert_eq!(encoded_function_selector, expected_function_selector);
    }
}
