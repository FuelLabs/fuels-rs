extern crate alloc;

use fuels_abigen_macro::wasm_abigen;

wasm_abigen!(
    no_name,
    r#"
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
);

pub fn the_fn() {
    use fuels_core::{abi_decoder::ABIDecoder, Tokenizable};
    use fuels_types::param_types::ParamType;
    let data = vec![
        0, 0, 0, 0, 0, 0, 3, 252, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175,
        175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175,
        175,
    ];

    let obj = ABIDecoder::decode_single(
        &ParamType::Struct {
            name: "".to_string(),
            fields: vec![
                ("unused".to_string(), ParamType::U64),
                ("unused".to_string(), ParamType::B256),
            ],
            generics: vec![],
        },
        &data,
    )
    .expect("Failed to decode");

    let a_struct = SomeEvent::from_token(obj).unwrap();

    assert_eq!(1020, a_struct.id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use webassembly_test::webassembly_test;

    #[webassembly_test]
    fn test() {
        the_fn();
    }
}
