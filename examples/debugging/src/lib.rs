#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::abi::unified_program::UnifiedProgramABI;
    use fuels::{
        core::codec::ABIDecoder,
        macros::abigen,
        types::{errors::Result, param_types::ParamType, SizedAsciiString},
    };

    #[test]
    fn encode_fn_selector() {
        use fuels::core::codec::encode_fn_selector;

        // ANCHOR: example_fn_selector
        // fn some_fn_name(arg1: Vec<str[3]>, arg2: u8)
        let fn_name = "some_fn_name";

        let selector = encode_fn_selector(fn_name);

        assert_eq!(
            selector,
            [0, 0, 0, 0, 0, 0, 0, 12, 115, 111, 109, 101, 95, 102, 110, 95, 110, 97, 109, 101]
        );
        // ANCHOR_END: example_fn_selector
    }

    #[test]
    fn decoded_debug_matches_rust_debug() -> Result<()> {
        abigen!(Contract(
            name = "MyContract",
            abi = "e2e/sway/types/contracts/generics/out/release/generics-abi.json"
        ));

        let json_abi_file = "../../e2e/sway/types/contracts/generics/out/release/generics-abi.json";
        let abi_file_contents = std::fs::read_to_string(json_abi_file)?;

        let parsed_abi = UnifiedProgramABI::from_json_abi(&abi_file_contents)?;

        let type_lookup = parsed_abi
            .types
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        let get_first_fn_argument = |fn_name: &str| {
            parsed_abi
                .functions
                .iter()
                .find(|abi_fun| abi_fun.name == fn_name)
                .expect("should be there")
                .inputs
                .first()
                .expect("should be there")
        };
        let decoder = ABIDecoder::default();

        {
            // simple struct with a single generic parameter
            let type_application = get_first_fn_argument("struct_w_generic");
            let param_type = ParamType::try_from_type_application(type_application, &type_lookup)?;

            let expected_struct = SimpleGeneric {
                single_generic_param: 123u64,
            };

            assert_eq!(
                format!("{expected_struct:?}"),
                decoder.decode_as_debug_str(&param_type, [0, 0, 0, 0, 0, 0, 0, 123].as_slice())?
            );
        }
        {
            // struct that delegates the generic param internally
            let type_application = get_first_fn_argument("struct_delegating_generic");
            let param_type = ParamType::try_from_type_application(type_application, &type_lookup)?;

            let expected_struct = PassTheGenericOn {
                one: SimpleGeneric {
                    single_generic_param: SizedAsciiString::<3>::try_from("abc")?,
                },
            };

            assert_eq!(
                format!("{expected_struct:?}"),
                decoder.decode_as_debug_str(&param_type, [97, 98, 99].as_slice())?
            );
        }
        {
            // enum with generic in variant
            let type_application = get_first_fn_argument("enum_w_generic");
            let param_type = ParamType::try_from_type_application(type_application, &type_lookup)?;

            let expected_enum = EnumWGeneric::B(10u64);

            assert_eq!(
                format!("{expected_enum:?}"),
                decoder.decode_as_debug_str(
                    &param_type,
                    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 10].as_slice()
                )?
            );
        }
        {
            // logged type
            let logged_type = parsed_abi
                .logged_types
                .as_ref()
                .expect("has logs")
                .first()
                .expect("has log");

            let param_type =
                ParamType::try_from_type_application(&logged_type.application, &type_lookup)?;

            let expected_u8 = 1;

            assert_eq!(
                format!("{expected_u8}"),
                decoder.decode_as_debug_str(&param_type, [1].as_slice())?
            );
        }

        Ok(())
    }
}
