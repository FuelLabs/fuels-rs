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
            // abi generated with: "e2e/sway/abi/wasm_contract"
            abi = r#"
            {
              "programType": "contract",
              "specVersion": "1",
              "encodingVersion": "1",
              "concreteTypes": [
                {
                  "type": "()",
                  "concreteTypeId": "2e38e77b22c314a449e91fafed92a43826ac6aa403ae6a8acb6cf58239fbaf5d"
                },
                {
                  "type": "enum SomeEnum<struct SomeStruct>",
                  "concreteTypeId": "744ffecb34b691a157f3f4b4657ea215fd23e3cc79fd7a3b7f15431751b46134",
                  "metadataTypeId": 1,
                  "typeArguments": [
                    "c672b07b5808bcc04715d73ca6d42eaabd332266144c1017c20833ef05a4a484"
                  ]
                },
                {
                  "type": "struct SomeStruct",
                  "concreteTypeId": "c672b07b5808bcc04715d73ca6d42eaabd332266144c1017c20833ef05a4a484",
                  "metadataTypeId": 3
                }
              ],
              "metadataTypes": [
                {
                  "type": "bool",
                  "metadataTypeId": 0
                },
                {
                  "type": "enum SomeEnum",
                  "metadataTypeId": 1,
                  "components": [
                    {
                      "name": "V1",
                      "typeId": "2e38e77b22c314a449e91fafed92a43826ac6aa403ae6a8acb6cf58239fbaf5d"
                    },
                    {
                      "name": "V2",
                      "typeId": 2
                    }
                  ],
                  "typeParameters": [
                    2
                  ]
                },
                {
                  "type": "generic T",
                  "metadataTypeId": 2
                },
                {
                  "type": "struct SomeStruct",
                  "metadataTypeId": 3,
                  "components": [
                    {
                      "name": "a",
                      "typeId": 4
                    },
                    {
                      "name": "b",
                      "typeId": 0
                    }
                  ]
                },
                {
                  "type": "u32",
                  "metadataTypeId": 4
                }
              ],
              "functions": [
                {
                  "inputs": [
                    {
                      "name": "_arg",
                      "concreteTypeId": "744ffecb34b691a157f3f4b4657ea215fd23e3cc79fd7a3b7f15431751b46134"
                    }
                  ],
                  "name": "test_function",
                  "output": "2e38e77b22c314a449e91fafed92a43826ac6aa403ae6a8acb6cf58239fbaf5d"
                }
              ],
              "loggedTypes": [],
              "messagesTypes": [],
              "configurables": []
         }
        "#
        ));

        let original = SomeEnum::V2(SomeStruct { a: 123, b: false });

        let bytes = ABIEncoder::default().encode(&[original.clone().into_token()])?;

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
            // abi generated with: "e2e/sway/abi/wasm_predicate"
            abi = r#"
            {
              "programType": "predicate",
              "specVersion": "1",
              "encodingVersion": "1",
              "concreteTypes": [
                {
                  "type": "bool",
                  "concreteTypeId": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903"
                },
                {
                  "type": "u64",
                  "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                }
              ],
              "metadataTypes": [],
              "functions": [
                {
                  "inputs": [
                    {
                      "name": "val",
                      "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0"
                    }
                  ],
                  "name": "main",
                  "output": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903",
                  "attributes": null
                }
              ],
              "loggedTypes": [],
              "messagesTypes": [],
              "configurables": [
                {
                  "name": "U64",
                  "concreteTypeId": "1506e6f44c1d6291cdf46395a8e573276a4fa79e8ace3fc891e092ef32d1b0a0",
                  "offset": 376
                }
              ]
            }
            "#
        ));

        let code = vec![
            26, 24, 48, 0, 116, 0, 0, 2, 0, 0, 0, 0, 0, 0, 1, 12, 93, 255, 192, 1, 16, 255, 255, 0,
            145, 0, 0, 8, 8, 235, 24, 0, 8, 228, 0, 8, 8, 224, 64, 0, 32, 248, 51, 0, 88, 251, 224,
            2, 8, 251, 224, 4, 116, 0, 0, 28, 26, 236, 8, 0, 145, 0, 0, 16, 113, 64, 0, 3, 97, 69,
            2, 0, 19, 73, 16, 0, 118, 72, 0, 6, 114, 72, 0, 2, 19, 69, 2, 128, 118, 68, 0, 1, 54,
            0, 0, 0, 97, 65, 2, 74, 116, 0, 0, 1, 97, 65, 2, 12, 95, 237, 0, 1, 8, 67, 176, 8, 26,
            233, 0, 0, 32, 248, 51, 0, 88, 251, 224, 2, 8, 251, 224, 4, 116, 0, 0, 32, 26, 67, 28,
            0, 26, 233, 0, 0, 32, 248, 51, 0, 88, 251, 224, 2, 8, 251, 224, 4, 116, 0, 0, 42, 26,
            67, 28, 0, 36, 64, 0, 0, 149, 0, 0, 15, 15, 8, 0, 0, 26, 236, 8, 0, 145, 0, 0, 16, 26,
            67, 16, 0, 26, 71, 128, 0, 26, 75, 224, 0, 95, 237, 0, 0, 26, 235, 176, 0, 32, 248, 51,
            0, 88, 251, 224, 2, 8, 251, 224, 4, 116, 0, 0, 11, 26, 67, 28, 0, 95, 237, 0, 1, 8, 67,
            176, 8, 114, 76, 0, 8, 4, 69, 4, 192, 26, 244, 0, 0, 146, 0, 0, 16, 26, 249, 32, 0,
            152, 8, 0, 0, 151, 0, 0, 15, 74, 248, 0, 0, 149, 0, 0, 31, 15, 8, 0, 0, 26, 236, 8, 0,
            26, 83, 16, 0, 26, 67, 224, 0, 93, 69, 64, 0, 93, 69, 16, 0, 93, 73, 64, 0, 114, 76, 0,
            8, 16, 73, 36, 192, 95, 81, 32, 0, 26, 245, 16, 0, 26, 249, 0, 0, 152, 8, 0, 0, 151, 0,
            0, 31, 74, 248, 0, 0, 149, 0, 0, 7, 15, 8, 0, 0, 26, 236, 8, 0, 26, 67, 16, 0, 26, 71,
            224, 0, 93, 72, 64, 0, 19, 65, 4, 128, 26, 245, 0, 0, 26, 249, 16, 0, 152, 8, 0, 0,
            151, 0, 0, 7, 74, 248, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128,
        ];
        let value = 129;

        let predicate_data = MyPredicateEncoder::default().encode_data(value)?;
        let configurables = MyPredicateConfigurables::default().with_U64(value)?;

        let predicate: Predicate = Predicate::from_code(code.clone())
            .with_data(predicate_data)
            .with_configurables(configurables);

        let mut expected_code = code.clone();
        *expected_code.last_mut().unwrap() = value as u8;

        assert_eq!(*predicate.code(), expected_code);

        let expected_address = Bech32Address::from_str(
            "fuel1c7rzx6ljxdz8egkcfjswffe7w8u06rm4nfvyu4lelyjua7qlcmdss9jkjm",
        )?;

        assert_eq!(*predicate.address(), expected_address);

        Ok(())
    }
}
