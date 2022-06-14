extern crate alloc;
use fuels_abigen_macro::wasm_abigen;

wasm_abigen!(
    no_name,
    r#"[
        {
            "type":"contract",
            "inputs":[
                {
                    "name":"SomeEvent",
                    "type":"struct SomeEvent",
                    "components": [
                        {
                            "name": "id",
                            "type": "u64"
                        },
                        {
                            "name": "account",
                            "type": "b256"
                        }
                    ]
                },
                {
                    "name":"AnotherEvent",
                    "type":"struct AnotherEvent",
                    "components": [
                        {
                            "name": "id",
                            "type": "u64"
                        },
                        {
                            "name": "hash",
                            "type": "b256"
                        },
                        {
                            "name": "bar",
                            "type": "bool"
                        }
                    ]
                }
            ],
            "name":"takes_struct",
            "outputs":[]
        }
    ]
    "#
);

pub fn the_fn() {
    use fuels_core::{abi_decoder::ABIDecoder, ParamType, Parameterize};
    let data = vec![
        0, 0, 0, 0, 0, 0, 3, 252, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175,
        175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175, 175,
        175,
    ];

    let obj =
        ABIDecoder::decode(&[ParamType::U64, ParamType::B256], &data).expect("Failed to decode");

    let a_struct = SomeEvent::new_from_tokens(&obj);

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
