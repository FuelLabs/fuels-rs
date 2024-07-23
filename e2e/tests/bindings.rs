use fuels::prelude::*;

pub fn null_contract_id() -> Bech32ContractId {
    // bech32 contract address that decodes to [0u8;32]
    "fuel1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsx2mt2"
        .parse()
        .unwrap()
}

mod hygiene {
    #[tokio::test]
    async fn setup_program_test_is_hygienic() {
        fuels::prelude::setup_program_test!(
            Wallets("wallet"),
            Abigen(Contract(
                name = "SimpleContract",
                project = "e2e/sway/bindings/simple_contract"
            )),
            Deploy(
                name = "simple_contract_instance",
                contract = "SimpleContract",
                wallet = "wallet"
            ),
        );
    }
}

#[tokio::test]
async fn compile_bindings_from_contract_file() {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "SimpleContract",
            project = "e2e/sway/bindings/simple_contract"
        )),
        Deploy(
            name = "simple_contract_instance",
            contract = "SimpleContract",
            wallet = "wallet"
        ),
    );

    let call_handler = simple_contract_instance
        .methods()
        .takes_int_returns_bool(42);

    let encoded_args = call_handler.call.encoded_args.unwrap();

    assert_eq!(encoded_args, [0, 0, 0, 42]);
}

#[tokio::test]
async fn compile_bindings_from_inline_contract() -> Result<()> {
    abigen!(Contract(
        name = "SimpleContract",
        // abi generated with: "e2e/sway/abi/simple_contract"
        abi = r#"
        {
          "programType": "contract",
          "specVersion": "1",
          "encodingVersion": "1",
          "concreteTypes": [
            {
              "type": "bool",
              "concreteTypeId": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903"
            },
            {
              "type": "u32",
              "concreteTypeId": "d7649d428b9ff33d188ecbf38a7e4d8fd167fa01b2e10fe9a8f9308e52f1d7cc"
            }
          ],
          "metadataTypes": [],
          "functions": [
            {
              "inputs": [
                {
                  "name": "_arg",
                  "concreteTypeId": "d7649d428b9ff33d188ecbf38a7e4d8fd167fa01b2e10fe9a8f9308e52f1d7cc"
                }
              ],
              "name": "takes_u32_returns_bool",
              "output": "b760f44fa5965c2474a3b471467a22c43185152129295af588b022ae50b50903",
              "attributes": null
            }
          ],
          "loggedTypes": [],
          "messagesTypes": [],
          "configurables": []
        }
        "#,
    ));

    let wallet = launch_provider_and_get_wallet().await?;

    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.methods().takes_u32_returns_bool(42_u32);
    let encoded_args = call_handler.call.encoded_args.unwrap();

    assert_eq!(encoded_args, [0, 0, 0, 42]);

    Ok(())
}

#[tokio::test]
async fn shared_types() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "ContractA",
                project = "e2e/sway/bindings/sharing_types/contract_a"
            ),
            Contract(
                name = "ContractB",
                project = "e2e/sway/bindings/sharing_types/contract_b"
            ),
        ),
        Deploy(
            name = "contract_a",
            contract = "ContractA",
            wallet = "wallet"
        ),
        Deploy(
            name = "contract_b",
            contract = "ContractB",
            wallet = "wallet"
        ),
    );
    {
        let methods = contract_a.methods();

        {
            let shared_struct_2 = SharedStruct2 {
                a: 11u32,
                b: SharedStruct1 { a: 12u32 },
            };
            let shared_enum = SharedEnum::a(10u64);
            let response = methods
                .uses_shared_type(shared_struct_2.clone(), shared_enum.clone())
                .call()
                .await?
                .value;

            assert_eq!(response, (shared_struct_2, shared_enum));
        }
        {
            let same_name_struct =
                abigen_bindings::contract_a_mod::StructSameNameButDifferentInternals { a: 13u32 };
            let same_name_enum =
                abigen_bindings::contract_a_mod::EnumSameNameButDifferentInternals::a(14u32);
            let response = methods
                .uses_types_that_share_only_names(same_name_struct.clone(), same_name_enum.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, (same_name_struct, same_name_enum));
        }
        {
            let arg = UniqueStructToContractA {
                a: SharedStruct2 {
                    a: 15u32,
                    b: SharedStruct1 { a: 5u8 },
                },
            };
            let response = methods
                .uses_shared_type_inside_owned_one(arg.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, arg);
        }
    }
    {
        let methods = contract_b.methods();

        {
            let shared_struct_2 = SharedStruct2 {
                a: 11u32,
                b: SharedStruct1 { a: 12u32 },
            };
            let shared_enum = SharedEnum::a(10u64);
            let response = methods
                .uses_shared_type(shared_struct_2.clone(), shared_enum.clone())
                .call()
                .await?
                .value;

            assert_eq!(response, (shared_struct_2, shared_enum));
        }
        {
            let same_name_struct =
                abigen_bindings::contract_b_mod::StructSameNameButDifferentInternals { a: [13u64] };
            let same_name_enum =
                abigen_bindings::contract_b_mod::EnumSameNameButDifferentInternals::a([14u64]);
            let response = methods
                .uses_types_that_share_only_names(same_name_struct.clone(), same_name_enum.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, (same_name_struct, same_name_enum));
        }
        {
            let arg = UniqueStructToContractB {
                a: SharedStruct2 {
                    a: 15u32,
                    b: SharedStruct1 { a: 5u8 },
                },
            };
            let response = methods
                .uses_shared_type_inside_owned_one(arg.clone())
                .call()
                .await?
                .value;
            assert_eq!(response, arg);
        }
    }

    Ok(())
}

#[cfg(feature = "test-type-paths")]
#[tokio::test]
async fn type_paths_respected() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "ContractA",
            project = "e2e/sway/bindings/type_paths"
        )),
        Deploy(
            name = "contract_a_instance",
            contract = "ContractA",
            wallet = "wallet"
        ),
    );
    {
        let contract_a_type =
            abigen_bindings::contract_a_mod::contract_a_types::VeryCommonNameStruct {
                another_field: 10u32,
            };

        let rtn = contract_a_instance
            .methods()
            .test_function(AWrapper {
                field: contract_a_type,
            })
            .call()
            .await?
            .value;

        let rtn_using_the_other_type =
            abigen_bindings::contract_a_mod::another_lib::VeryCommonNameStruct { field_a: 10u32 };

        assert_eq!(rtn, rtn_using_the_other_type);
    }

    Ok(())
}
