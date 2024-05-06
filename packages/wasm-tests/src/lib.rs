extern crate alloc;

#[cfg(test)]
mod tests {
    use std::{default::Default, str::FromStr};

    use fuels::{
        accounts::predicate::Predicate,
        core::{codec::ABIEncoder, traits::Tokenizable},
        macros::wasm_abigen,
        types::{bech32::Bech32Address, errors::Result},
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn decoding_and_encoding() -> Result<()> {
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

        let original = SomeEnum::V2(SomeStruct { a: 123, b: false });

        let bytes = ABIEncoder::default()
            .encode(&[original.clone().into_token()])?
            .resolve(0);

        let expected_bytes = [
            0, 0, 0, 0, 0, 0, 0, 1, // enum discriminant
            0, 0, 0, 123, 0, // SomeStruct
        ]
        .to_vec();

        assert_eq!(expected_bytes, bytes);

        let reconstructed = bytes.try_into().unwrap();

        assert_eq!(original, reconstructed);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn predicate_abigen() -> Result<()> {
        wasm_abigen!(Predicate(
            name = "MyPredicate",
            abi = r#"
                    {
                      "types": [
                        {
                          "typeId": 0,
                          "type": "bool",
                          "components": null,
                          "typeParameters": null
                        },
                        {
                          "typeId": 1,
                          "type": "u64",
                          "components": null,
                          "typeParameters": null
                        }
                      ],
                      "functions": [
                        {
                          "inputs": [
                            {
                              "name": "arg",
                              "type": 1,
                              "typeArguments": null
                            }
                          ],
                          "name": "main",
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
                      "configurables": [
                        {
                          "name": "U64",
                          "configurableType": {
                            "name": "",
                            "type": 1,
                            "typeArguments": null
                          },
                          "offset": 100
                        }
                      ]
                    }"#
        ));

        let code = vec![
            116, 0, 0, 3, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 93, 252, 192, 1, 16, 255, 243, 0,
            26, 236, 80, 0, 145, 0, 0, 0, 113, 68, 0, 3, 97, 73, 17, 1, 118, 72, 0, 2, 97, 65, 17,
            13, 116, 0, 0, 7, 114, 76, 0, 2, 19, 73, 36, 192, 90, 73, 32, 1, 118, 72, 0, 2, 97, 65,
            17, 31, 116, 0, 0, 1, 36, 0, 0, 0, 93, 65, 0, 0, 93, 71, 240, 0, 19, 65, 4, 64, 36, 64,
            0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42,
        ];
        let value = 128;

        let predicate_data = MyPredicateEncoder::default().encode_data(value)?;
        let configurables = MyPredicateConfigurables::default().with_U64(value)?;

        let predicate: Predicate = Predicate::from_code(code.clone())
            .with_data(predicate_data)
            .with_configurables(configurables);

        let mut expected_code = code.clone();
        *expected_code.last_mut().unwrap() = value as u8;

        assert_eq!(*predicate.code(), expected_code);

        let expected_address = Bech32Address::from_str(
            "fuel14z2xsxcp47z9zfhj9atrmd66ujvwy8ujgn4j0xsh95fjh2px4mcq4f7k3w",
        )?;

        assert_eq!(*predicate.address(), expected_address);

        Ok(())
    }
}
