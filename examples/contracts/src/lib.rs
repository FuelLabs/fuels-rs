#[cfg(test)]
mod tests {
    use fuels::prelude::*;
    use fuels_abigen_macro::abigen;

    #[tokio::test]
    async fn example_workflow() {
        // Generates the bindings from the an ABI definition inline.
        // The generated bindings can be accessed through `MyContract`.
        abigen!(
        MyContract,
        // This path is relative to the workspace (repository) root
        "packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test-abi.json"
    );

        let wallet = launch_provider_and_get_single_wallet().await;

        let contract_id = Contract::deploy(
            // This path is relative to the current crate (examples/contracts)
            "../../packages/fuels-abigen-macro/tests/test_projects/contract_test/out/debug/contract_test.bin",
            &wallet,
            TxParameters::default(),
        )
        .await
        .unwrap();

        println!("Contract deployed @ {:x}", contract_id);
        let contract_instance = MyContract::new(contract_id.to_string(), wallet);

        let result = contract_instance
            .initialize_counter(42) // Build the ABI call
            .tx_params(TxParameters::new(None, Some(1_000_000), None, None))
            .call() // Perform the network call
            .await
            .unwrap();

        assert_eq!(42, result.value);

        let result = contract_instance
            .increment_counter(10)
            .call()
            .await
            .unwrap();

        assert_eq!(52, result.value);
    }
}
