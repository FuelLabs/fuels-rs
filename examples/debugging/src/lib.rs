#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::abi::program::ProgramABI;
    #[cfg(not(feature = "experimental"))]
    use fuels::core::codec::{calldata, fn_selector};
    use fuels::{
        core::codec::ABIDecoder,
        macros::abigen,
        types::{errors::Result, param_types::ParamType, SizedAsciiString},
    };

    #[cfg(not(feature = "experimental"))]
    #[test]
    fn get_a_fn_selector() {
        use fuels::core::{codec::resolve_fn_selector, traits::Parameterize};

        // ANCHOR: example_fn_selector
        // fn some_fn_name(arg1: Vec<str[3]>, arg2: u8)
        let fn_name = "some_fn_name";
        let inputs = [Vec::<SizedAsciiString<3>>::param_type(), u8::param_type()];

        let selector = resolve_fn_selector(fn_name, &inputs);

        assert_eq!(selector, [0, 0, 0, 0, 7, 161, 3, 203]);
        // ANCHOR_END: example_fn_selector
    }

    #[cfg(not(feature = "experimental"))]
    #[test]
    fn a_fn_selector_from_json_abi() -> Result<()> {
        use fuels::core::codec::resolve_fn_selector;

        let json_abi_file =
            "../../packages/fuels/tests/contracts/contract_test/out/debug/contract_test-abi.json";
        let abi_file_contents = std::fs::read_to_string(json_abi_file)?;

        // ANCHOR: example_fn_selector_json
        let abi: ProgramABI = serde_json::from_str(&abi_file_contents)?;

        let type_lookup = abi
            .types
            .into_iter()
            .map(|a_type| (a_type.type_id, a_type))
            .collect::<HashMap<_, _>>();

        let a_fun = abi
            .functions
            .into_iter()
            .find(|fun| fun.name == "array_of_structs")
            .unwrap();

        let inputs = a_fun
            .inputs
            .into_iter()
            .map(|type_appl| ParamType::try_from_type_application(&type_appl, &type_lookup))
            .collect::<Result<Vec<_>>>()?;

        let selector = resolve_fn_selector(&a_fun.name, &inputs);

        assert_eq!(selector, [0, 0, 0, 0, 39, 152, 108, 146,]);
        // ANCHOR_END: example_fn_selector_json

        Ok(())
    }

    #[cfg(not(feature = "experimental"))]
    #[test]
    fn test_macros() -> Result<()> {
        let function_selector = fn_selector!(initialize_counter(u64));
        let call_data = calldata!(42u64)?;

        assert_eq!(vec![0, 0, 0, 0, 171, 100, 229, 242], function_selector);
        assert_eq!(vec![0, 0, 0, 0, 0, 0, 0, 42], call_data);

        Ok(())
    }

    #[test]
    fn decoded_debug_matches_rust_debug() -> Result<()> {
        abigen!(Contract(
            name = "MyContract",
            abi = "packages/fuels/tests/types/contracts/generics/out/debug/generics-abi.json"
        ));

        let json_abi_file =
            "../../packages/fuels/tests/types/contracts/generics/out/debug/generics-abi.json";
        let abi_file_contents = std::fs::read_to_string(json_abi_file)?;

        let parsed_abi: ProgramABI = serde_json::from_str(&abi_file_contents)?;

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
                decoder.decode_as_debug_str(&param_type, &[0, 0, 0, 0, 0, 0, 0, 123])?
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
                decoder.decode_as_debug_str(&param_type, &[97, 98, 99])?
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
                    &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 10]
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

            #[cfg(not(feature = "experimental"))]
            let data = [0, 0, 0, 0, 0, 0, 0, 1];
            #[cfg(feature = "experimental")]
            let data = [1];

            assert_eq!(
                format!("{expected_u8}"),
                decoder.decode_as_debug_str(&param_type, &data)?
            );
        }

        Ok(())
    }
}
