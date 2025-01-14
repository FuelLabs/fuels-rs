#[cfg(test)]
mod tests {
    use fuels::prelude::Result;

    #[tokio::test]
    #[allow(unused_variables)]
    async fn transform_json_to_bindings() -> Result<()> {
        use fuels::test_helpers::launch_provider_and_get_wallet;
        let wallet = launch_provider_and_get_wallet().await?;
        {
            // ANCHOR: use_abigen
            use fuels::prelude::*;
            // Replace with your own JSON abi path (relative to the root of your crate)
            abigen!(Contract(
                name = "MyContractName",
                abi = "examples/rust_bindings/src/abi.json"
            ));
            // ANCHOR_END: use_abigen
        }

        {
            // ANCHOR: abigen_with_string
            use fuels::prelude::*;
            abigen!(Contract(
                name = "MyContract",
                abi = r#"
            {
              "programType": "contract",
              "specVersion": "1",
              "encodingVersion": "1",
              "concreteTypes": [
                {
                  "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0",
                  "type": "u64"
                }
              ],
              "functions": [
                {
                  "inputs": [
                    {
                      "name": "value",
                      "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                    }
                  ],
                  "name": "initialize_counter",
                  "output": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                },
                {
                  "inputs": [
                    {
                      "name": "value",
                      "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                    }
                  ],
                  "name": "increment_counter",
                  "output": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                }
              ],
              "metadataTypes": []
            }
            "#
            ));
            // ANCHOR_END: abigen_with_string
        }
        Ok(())
    }
}
