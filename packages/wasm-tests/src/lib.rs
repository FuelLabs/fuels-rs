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
    let a_struct = SomeEvent {
        id: 20,
        account: Default::default(),
    };

    println!("It works! {}", a_struct.id);
}
