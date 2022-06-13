use fuels::prelude::{launch_provider_and_get_single_wallet, Contract, TxParameters};
use fuels_abigen_macro::abigen;
use fuels_core::tx::Salt;

#[tokio::test]
#[should_panic]
async fn deploy_panics_on_non_binary_file() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    // Should panic as we are passing in a JSON instead of BIN
    Contract::deploy(
        "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
        &wallet,
        TxParameters::default(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[should_panic]
async fn deploy_with_salt_panics_on_non_binary_file() {
    abigen!(
        MyContract,
        "packages/fuels-abigen-macro/tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json"
    );

    let wallet = launch_provider_and_get_single_wallet().await;

    // Should panic as we are passing in a JSON instead of BIN
    Contract::deploy_with_salt(
        "tests/test_projects/contract_output_test/out/debug/contract_output_test-abi.json",
        &wallet,
        TxParameters::default(),
        Salt::default(),
    )
    .await
    .unwrap();
}
