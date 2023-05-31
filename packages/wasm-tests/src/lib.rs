extern crate alloc;

#[cfg(test)]
mod tests {
    use fuels::{
        core::{codec::ABIEncoder, traits::Tokenizable},
        macros::wasm_abigen,
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
                          "name": "v1",
                          "type": 0,
                          "typeArguments": null
                        },
                        {
                          "name": "v2",
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
    fn decoding_and_encoding() {
        let original = SomeEnum::v2(SomeStruct { a: 123, b: false });

        let bytes = ABIEncoder::encode(&[original.clone().into_token()])
            .unwrap()
            .resolve(0);

        let reconstructed = bytes.try_into().unwrap();

        assert_eq!(original, reconstructed);
    }
}
