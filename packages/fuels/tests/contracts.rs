#[allow(unused_imports)]
use std::future::Future;
use std::vec;

use fuels::{
    accounts::{predicate::Predicate, Account},
    core::codec::{calldata, fn_selector},
    prelude::*,
    types::Bits256,
};
use fuels_core::codec::DecoderConfig;

#[tokio::test]
async fn test_multiple_args() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "packages/fuels/tests/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet"
        ),
    );

    dbg!("halil");
    // Make sure we can call the contract with multiple arguments
    let contract_methods = contract_instance.methods();
    let response = contract_methods.get(5, 6).call().await?;

    // assert_eq!(response.value, 11);

    // let t = MyType { x: 5, y: 6 };
    // let response = contract_methods.get_alt(t.clone()).call().await?;
    // assert_eq!(response.value, t);

    // let response = contract_methods.get_single(5).call().await?;
    // assert_eq!(response.value, 5);
    Ok(())
}
