#[cfg(test)]
mod tests {
    use fuels::{
        core::try_from_bytes,
        prelude::{AssetId, ContractId, Result},
    };

    #[tokio::test]
    #[allow(unused_variables)]
    async fn transform_json_to_bindings() -> Result<()> {
        use fuels::test_helpers::launch_provider_and_get_wallet;
        let wallet = launch_provider_and_get_wallet().await;
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
                "types": [
                  {
                    "typeId": 0,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  }
                ],
                "functions": [
                  {
                    "inputs": [
                      {
                        "name": "value",
                        "type": 0,
                        "typeArguments": null
                      }
                    ],
                    "name": "initialize_counter",
                    "output": {
                      "name": "",
                      "type": 0,
                      "typeArguments": null
                    }
                  },
                  {
                    "inputs": [
                      {
                        "name": "value",
                        "type": 0,
                        "typeArguments": null
                      }
                    ],
                    "name": "increment_counter",
                    "output": {
                      "name": "",
                      "type": 0,
                      "typeArguments": null
                    }
                  }
                ]
              }
            "#
            ));
            // ANCHOR_END: abigen_with_string
        }
        Ok(())
    }

    #[test]
    fn manual_decode_of_native_types() -> Result<()> {
        // ANCHOR: manual_decode_native
        let contract_id_bytes = [0xFF; 32];
        let contract_id = ContractId::new(contract_id_bytes);

        let asset_id_bytes = [0xFF; 32];
        let asset_id = AssetId::new(asset_id_bytes);

        let bytes: Vec<u8> = [contract_id_bytes, asset_id_bytes].concat();
        let expected: (ContractId, AssetId) = try_from_bytes(&bytes)?;

        assert_eq!(expected, (contract_id, asset_id));
        // ANCHOR_END: manual_decode_native
        Ok(())
    }
}
