use fuels::prelude::*;

#[tokio::test]
async fn test_alias() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/alias"
        )),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    // Make sure we can call the contract with multiple arguments
    let contract_methods = contract_instance.methods();
    use abigen_bindings::my_contract_mod::MyAlias;
    let response = contract_methods.with_myalias(MyAlias::zeroed()).call().await?;

    assert_eq!(response.value, MyAlias::zeroed());

    Ok(())
}
