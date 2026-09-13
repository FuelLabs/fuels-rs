// An empty no_std contract exercises the shared ABI parser without SDK bindings.
fuels_macros::wasm_abigen!(Contract(name = "Nested", abi = "abi.json"));

mod inline {
    fuels_macros::wasm_abigen!(Contract(
        name = "Inline",
        abi = r#"{
            "programType": "contract",
            "specVersion": "1",
            "encodingVersion": "1",
            "concreteTypes": [],
            "metadataTypes": [],
            "functions": []
        }"#
    ));
}
