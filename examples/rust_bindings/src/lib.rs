#[allow(unused_imports)]
use fuels::prelude::Error;

#[tokio::test]
#[allow(unused_variables)]
async fn transform_json_to_bindings() -> Result<(), Error> {
    use fuels::test_helpers::launch_provider_and_get_wallet;
    let wallet = launch_provider_and_get_wallet().await;
    // ANCHOR: use_abigen
    use fuels::prelude::*;
    // Replace with your own JSON abi path (relative to the root of your crate)
    abigen!(MyContractName, "examples/rust_bindings/src/abi.json");
    // ANCHOR_END: use_abigen

    // ANCHOR: abigen_with_string
    // Don't forget to import the `abigen` macro as above
    abigen!(
        MyContract,
        r#"
    [
        {
            "type": "function",
            "inputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ],
            "name": "initialize_counter",
            "outputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ]
        },
        {
            "type": "function",
            "inputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ],
            "name": "increment_counter",
            "outputs": [
                {
                    "name": "arg",
                    "type": "u64"
                }
            ]
        }
    ]
    "#
    );
    // ANCHOR_END: abigen_with_string
    Ok(())
}
