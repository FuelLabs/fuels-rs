#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::program_abi::ProgramABI;
    use fuels::{
        core::{
            codec::{calldata, fn_selector, resolve_fn_selector},
            traits::Parameterize,
        },
        types::{errors::Result, param_types::ParamType, SizedAsciiString},
    };

    #[test]
    fn get_a_fn_selector() {
        // ANCHOR: example_fn_selector
        // fn some_fn_name(arg1: Vec<str[3]>, arg2: u8)
        let fn_name = "some_fn_name";
        let inputs = [Vec::<SizedAsciiString<3>>::param_type(), u8::param_type()];

        let selector = resolve_fn_selector(fn_name, &inputs);

        assert_eq!(selector, [0, 0, 0, 0, 7, 161, 3, 203]);
        // ANCHOR_END: example_fn_selector
    }

    #[test]
    fn a_fn_selector_from_json_abi() -> Result<()> {
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

    #[test]
    fn test_macros() {
        let function_selector = fn_selector!(initialize_counter(u64));
        let call_data = calldata!(42u64);

        assert_eq!(vec![0, 0, 0, 0, 171, 100, 229, 242], function_selector);
        assert_eq!(vec![0, 0, 0, 0, 0, 0, 0, 42], call_data);
    }
}
