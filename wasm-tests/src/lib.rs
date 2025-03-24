extern crate alloc;

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use std::{default::Default, str::FromStr};

    use fuels::{
        accounts::predicate::Predicate,
        core::{codec::ABIEncoder, traits::Tokenizable},
        macros::wasm_abigen,
        programs::debug::ScriptType,
        types::{AssetId, bech32::Bech32Address, errors::Result},
    };
    use fuels_core::codec::abi_formatter::ABIFormatter;
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

    #[wasm_bindgen_test]
    fn can_decode_a_contract_calling_script() -> Result<()> {
        let script = hex::decode(
            "724028d8724428b05d451000724828b82d41148a724029537244292b5d451000724829332d41148a24040000",
        )?;
        let script_data = hex::decode(
            "000000000000000a00000000000000000000000000000000000000000000000000000000000000001e62ecaa5c32f1e51954f46149d5e542472bdba45838199406464af46ab147ed000000000000290800000000000029260000000000000016636865636b5f7374727563745f696e746567726974790000000201000000000000001400000000000000000000000000000000000000000000000000000000000000001e62ecaa5c32f1e51954f46149d5e542472bdba45838199406464af46ab147ed000000000000298300000000000029a20000000000000017695f616d5f63616c6c65645f646966666572656e746c7900000002011e62ecaa5c32f1e51954f46149d5e542472bdba45838199406464af46ab147ed000000000000007b00000000000001c8",
        )?;

        let abi = r#"{
            "programType": "contract",
            "specVersion": "1",
            "encodingVersion": "1",
            "concreteTypes": [
                {
                "type": "()",
                "concreteTypeId": "2e38e77b22c314a449e91fafed92a43826ac6aa403ae6a8acb6cf58239fbaf5d"
                },
                {
                "type": "bool",
                "concreteTypeId": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903"
                },
                {
                "type": "struct AllStruct",
                "concreteTypeId": "91804f0112892169cddf041007c9f16f95281d45c3f363e544c33dffc8179266",
                "metadataTypeId": 1
                },
                {
                "type": "struct CallData",
                "concreteTypeId": "c1b2644ef8de5c5b7a95aaadf3f5cedd40f42286d459bcd051c3cc35fa1ce5ec",
                "metadataTypeId": 2
                },
                {
                "type": "struct MemoryAddress",
                "concreteTypeId": "0b7b6a791f80f65fe493c3e0d0283bf8206871180c9b696797ff0098ff63b474",
                "metadataTypeId": 3
                }
            ],
            "metadataTypes": [
                {
                "type": "b256",
                "metadataTypeId": 0
                },
                {
                "type": "struct AllStruct",
                "metadataTypeId": 1,
                "components": [
                    {
                    "name": "some_struct",
                    "typeId": 4
                    }
                ]
                },
                {
                "type": "struct CallData",
                "metadataTypeId": 2,
                "components": [
                    {
                    "name": "memory_address",
                    "typeId": 3
                    },
                    {
                    "name": "num_coins_to_forward",
                    "typeId": 7
                    },
                    {
                    "name": "asset_id_of_coins_to_forward",
                    "typeId": 5
                    },
                    {
                    "name": "amount_of_gas_to_forward",
                    "typeId": 7
                    }
                ]
                },
                {
                "type": "struct MemoryAddress",
                "metadataTypeId": 3,
                "components": [
                    {
                    "name": "contract_id",
                    "typeId": 5
                    },
                    {
                    "name": "function_selector",
                    "typeId": 7
                    },
                    {
                    "name": "function_data",
                    "typeId": 7
                    }
                ]
                },
                {
                "type": "struct SomeStruct",
                "metadataTypeId": 4,
                "components": [
                    {
                    "name": "field",
                    "typeId": 6
                    },
                    {
                    "name": "field_2",
                    "typeId": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903"
                    }
                ]
                },
                {
                "type": "struct std::contract_id::ContractId",
                "metadataTypeId": 5,
                "components": [
                    {
                    "name": "bits",
                    "typeId": 0
                    }
                ]
                },
                {
                "type": "u32",
                "metadataTypeId": 6
                },
                {
                "type": "u64",
                "metadataTypeId": 7
                }
            ],
            "functions": [
                {
                "inputs": [
                    {
                    "name": "arg",
                    "concreteTypeId": "91804f0112892169cddf041007c9f16f95281d45c3f363e544c33dffc8179266"
                    }
                ],
                "name": "check_struct_integrity",
                "output": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903",
                "attributes": [
                    {
                    "name": "payable",
                    "arguments": []
                    }
                ]
                },
                {
                "inputs": [],
                "name": "get_struct",
                "output": "91804f0112892169cddf041007c9f16f95281d45c3f363e544c33dffc8179266",
                "attributes": null
                },
                {
                "inputs": [
                    {
                    "name": "arg1",
                    "concreteTypeId": "91804f0112892169cddf041007c9f16f95281d45c3f363e544c33dffc8179266"
                    },
                    {
                    "name": "arg2",
                    "concreteTypeId": "0b7b6a791f80f65fe493c3e0d0283bf8206871180c9b696797ff0098ff63b474"
                    }
                ],
                "name": "i_am_called_differently",
                "output": "2e38e77b22c314a449e91fafed92a43826ac6aa403ae6a8acb6cf58239fbaf5d",
                "attributes": [
                    {
                    "name": "payable",
                    "arguments": []
                    }
                ]
                },
                {
                "inputs": [
                    {
                    "name": "call_data",
                    "concreteTypeId": "c1b2644ef8de5c5b7a95aaadf3f5cedd40f42286d459bcd051c3cc35fa1ce5ec"
                    }
                ],
                "name": "nested_struct_with_reserved_keyword_substring",
                "output": "c1b2644ef8de5c5b7a95aaadf3f5cedd40f42286d459bcd051c3cc35fa1ce5ec",
                "attributes": null
                }
            ],
            "loggedTypes": [],
            "messagesTypes": [],
            "configurables": []
        }"#;

        let decoder = ABIFormatter::from_json_abi(abi)?;

        // when
        let script_type = ScriptType::detect(&script, &script_data)?;

        // then
        let ScriptType::ContractCall(call_descriptions) = script_type else {
            panic!("expected a contract call")
        };

        assert_eq!(call_descriptions.len(), 2);

        let call_description = &call_descriptions[0];

        let expected_contract_id =
            "1e62ecaa5c32f1e51954f46149d5e542472bdba45838199406464af46ab147ed".parse()?;
        assert_eq!(call_description.contract_id, expected_contract_id);
        assert_eq!(call_description.amount, 10);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "check_struct_integrity"
        );
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                &call_description.encoded_args
            )?,
            vec!["AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }"]
        );

        let call_description = &call_descriptions[1];

        assert_eq!(call_description.contract_id, expected_contract_id);
        assert_eq!(call_description.amount, 20);
        assert_eq!(call_description.asset_id, AssetId::default());
        assert_eq!(
            call_description.decode_fn_selector().unwrap(),
            "i_am_called_differently"
        );
        assert!(call_description.gas_forwarded.is_none());

        assert_eq!(
            decoder.decode_fn_args(
                &call_description.decode_fn_selector().unwrap(),
                &call_description.encoded_args
            )?,
            vec![
                "AllStruct { some_struct: SomeStruct { field: 2, field_2: true } }",
                "MemoryAddress { contract_id: std::contract_id::ContractId { bits: Bits256([30, 98, 236, 170, 92, 50, 241, 229, 25, 84, 244, 97, 73, 213, 229, 66, 71, 43, 219, 164, 88, 56, 25, 148, 6, 70, 74, 244, 106, 177, 71, 237]) }, function_selector: 123, function_data: 456 }"
            ]
        );

        Ok(())
    }
}
