#[cfg(test)]
mod tests {
    use fuel_abi_types::program_abi::ProgramABI;
    use fuels::core::code_gen::function_selector::resolve_fn_selector;
    use fuels::core::Parameterize;
    use fuels::prelude::SizedAsciiString;
    use fuels::types::param_types::ParamType;
    use std::collections::HashMap;

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
    fn a_fn_selector_from_json_abi() -> anyhow::Result<()> {
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
            .collect::<Result<Vec<_>, _>>()?;

        let selector = resolve_fn_selector(&a_fun.name, &inputs);

        assert_eq!(selector, [0, 0, 0, 0, 39, 152, 108, 146,]);
        // ANCHOR_END: example_fn_selector_json

        Ok(())
    }
}
