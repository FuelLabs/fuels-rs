extern crate alloc;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use fuels::{
        accounts::predicate::Predicate,
        core::{codec::ABIEncoder, traits::Tokenizable, Configurables},
        macros::wasm_abigen,
        types::{bech32::Bech32Address, errors::Result},
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_abigen!(Contract(
        name = "no_name",
        abi = r#"
                {
                  "types": [
                    {
                      "typeId": 0,
                      "type": "()",
                      "components": [],
                      "typeParameters": null
                    },
                    {
                      "typeId": 1,
                      "type": "bool",
                      "components": null,
                      "typeParameters": null
                    },
                    {
                      "typeId": 2,
                      "type": "enum SomeEnum",
                      "components": [
                        {
                          "name": "V1",
                          "type": 0,
                          "typeArguments": null
                        },
                        {
                          "name": "V2",
                          "type": 3,
                          "typeArguments": null
                        }
                      ],
                      "typeParameters": [
                        3
                      ]
                    },
                    {
                      "typeId": 3,
                      "type": "generic T",
                      "components": null,
                      "typeParameters": null
                    },
                    {
                      "typeId": 4,
                      "type": "struct SomeStruct",
                      "components": [
                        {
                          "name": "a",
                          "type": 5,
                          "typeArguments": null
                        },
                        {
                          "name": "b",
                          "type": 1,
                          "typeArguments": null
                        }
                      ],
                      "typeParameters": null
                    },
                    {
                      "typeId": 5,
                      "type": "u32",
                      "components": null,
                      "typeParameters": null
                    }
                  ],
                  "functions": [
                    {
                      "inputs": [
                        {
                          "name": "arg",
                          "type": 2,
                          "typeArguments": [
                            {
                              "name": "",
                              "type": 4,
                              "typeArguments": null
                            }
                          ]
                        }
                      ],
                      "name": "test_function",
                      "output": {
                        "name": "",
                        "type": 0,
                        "typeArguments": null
                      },
                      "attributes": null
                    }
                  ],
                  "loggedTypes": [],
                  "messagesTypes": [],
                  "configurables": []
        }"#
    ));

    #[wasm_bindgen_test]
    fn decoding_and_encoding() -> Result<()> {
        let original = SomeEnum::V2(SomeStruct { a: 123, b: false });

        let bytes = ABIEncoder::encode(&[original.clone().into_token()])?.resolve(0);

        let reconstructed = bytes.try_into().unwrap();

        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn predicate_from_code_with_configurables() -> Result<()> {
        let code = vec![0, 1, 2, 3];
        let chain_id = 0;
        let configurables = Configurables::new(vec![(1, vec![5, 6])]);

        let predicate = Predicate::from_code(code, chain_id).with_configurables(configurables);

        let expected_code = vec![0u8, 5, 6, 3];
        assert_eq!(*predicate.code(), expected_code);

        let expected_address = Bech32Address::from_str(
            "fuel1cc9jrur8n535cnh205qdjd8jpxzhy8efpxr9zfjm8lyzjspa262scpm0ww",
        )?;
        assert_eq!(*predicate.address(), expected_address);

        Ok(())
    }
}
