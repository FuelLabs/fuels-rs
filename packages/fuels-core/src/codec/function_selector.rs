use sha2::{Digest, Sha256};

use crate::types::{param_types::ParamType, ByteArray};

/// Given a function name and its inputs  will return a ByteArray representing
/// the function selector as specified in the Fuel specs.
pub fn resolve_fn_selector(name: &str, inputs: &[ParamType]) -> ByteArray {
    let fn_signature = resolve_fn_signature(name, inputs);

    first_four_bytes_of_sha256_hash(&fn_signature)
}

fn resolve_fn_signature(name: &str, inputs: &[ParamType]) -> String {
    let fn_args = resolve_args(inputs);

    format!("{name}({fn_args})")
}

fn resolve_args(arg: &[ParamType]) -> String {
    arg.iter().map(resolve_arg).collect::<Vec<_>>().join(",")
}

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
            let field_params = resolve_args(fields);
            let gen_params = if !gen_params.is_empty() {
                format!("<{gen_params}>")
            } else {
                gen_params
            };
            format!("s{gen_params}({field_params})")
        }
        ParamType::Enum {
            variants: fields,
            generics,
            ..
        } => {
            let gen_params = resolve_args(generics);
            let field_params = resolve_args(fields.param_types());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::enum_variants::EnumVariants;

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
        let fields = vec![ParamType::U64, ParamType::U32];
        let generics = vec![ParamType::U32];
        let inputs = [ParamType::Struct { fields, generics }];

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
        let types = vec![ParamType::U64, ParamType::U32];
        let variants = EnumVariants::new(types).unwrap();
        let generics = vec![ParamType::U32];
        let inputs = [ParamType::Enum { variants, generics }];

        let selector = resolve_fn_signature("some_fun", &inputs);

        assert_eq!(selector, format!("some_fun(e<u32>(u64,u32))"));
    }

    #[test]
    fn ultimate_test() {
        let fields = vec![ParamType::Struct {
            fields: vec![ParamType::StringArray(2)],
            generics: vec![ParamType::StringArray(2)],
        }];
        let struct_a = ParamType::Struct {
            fields,
            generics: vec![ParamType::StringArray(2)],
        };

        let fields = vec![ParamType::Array(Box::new(struct_a.clone()), 2)];
        let struct_b = ParamType::Struct {
            fields,
            generics: vec![struct_a],
        };

        let fields = vec![ParamType::Tuple(vec![struct_b.clone(), struct_b.clone()])];
        let struct_c = ParamType::Struct {
            fields,
            generics: vec![struct_b],
        };

        let types = vec![ParamType::U64, struct_c.clone()];
        let fields = vec![
            ParamType::Tuple(vec![
                ParamType::Array(Box::new(ParamType::B256), 2),
                ParamType::StringArray(2),
            ]),
            ParamType::Tuple(vec![
                ParamType::Array(
                    Box::new(ParamType::Enum {
                        variants: EnumVariants::new(types).unwrap(),
                        generics: vec![struct_c],
                    }),
                    1,
                ),
                ParamType::U32,
            ]),
        ];

        let inputs = [ParamType::Struct {
            fields,
            generics: vec![ParamType::StringArray(2), ParamType::B256],
        }];

        let selector = resolve_fn_signature("complex_test", &inputs);

        assert_eq!(selector, "complex_test(s<str[2],b256>((a[b256;2],str[2]),(a[e<s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])))>(u64,s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]))));1],u32)))");
    }
}
