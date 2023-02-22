extern crate alloc;

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::wasm_bindgen_test;

    use fuels_core::abi_encoder::ABIEncoder;
    use fuels_macros::wasm_abigen;
    use fuels_types::traits::Tokenizable;
    use fuels_types::Bits256;

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
            "type": "b256",
            "components": null,
            "typeParameters": null
          },
          {
            "typeId": 2,
            "type": "bool",
            "components": null,
            "typeParameters": null
          },
          {
            "typeId": 3,
            "type": "struct AnotherEvent",
            "components": [
              {
                "name": "id",
                "type": 5,
                "typeArguments": null
              },
              {
                "name": "hash",
                "type": 1,
                "typeArguments": null
              },
              {
                "name": "bar",
                "type": 2,
                "typeArguments": null
              }
            ],
            "typeParameters": null
          },
          {
            "typeId": 4,
            "type": "struct SomeEvent",
            "components": [
              {
                "name": "id",
                "type": 5,
                "typeArguments": null
              },
              {
                "name": "account",
                "type": 1,
                "typeArguments": null
              }
            ],
            "typeParameters": null
          },
          {
            "typeId": 5,
            "type": "u64",
            "components": null,
            "typeParameters": null
          }
        ],
        "functions": [
          {
            "inputs": [
              {
                "name": "e1",
                "type": 4,
                "typeArguments": null
              },
              {
                "name": "e2",
                "type": 3,
                "typeArguments": null
              }
            ],
            "name": "takes_struct",
            "output": {
              "name": "",
              "type": 0,
              "typeArguments": null
            }
          }
        ],
        "loggedTypes": []
      }
    "#
    ));

    #[wasm_bindgen_test]
    fn decoding_and_encoding() {
        let original_event = AnotherEvent {
            id: 2,
            hash: Bits256([2; 32]),
            bar: true,
        };

        let bytes = ABIEncoder::encode(&[original_event.clone().into_token()])
            .unwrap()
            .resolve(0);

        let reconstructed_event: AnotherEvent = bytes.try_into().unwrap();

        assert_eq!(original_event, reconstructed_event);
    }
}
