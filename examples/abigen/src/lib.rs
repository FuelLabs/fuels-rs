#[cfg(test)]
mod tests {
    use fuels::prelude::*;

    #[tokio::test]
    async fn example_of_abigen_usage() -> Result<()> {
        // ANCHOR: multiple_abigen_program_types
        abigen!(
            Contract(name="ContractA", abi="packages/fuels/tests/bindings/sharing_types/contract_a/out/debug/contract_a-abi.json"),
            Contract(name="ContractB", abi="packages/fuels/tests/bindings/sharing_types/contract_b/out/debug/contract_b-abi.json"),
            Script(name="MyScript", abi="packages/fuels/tests/scripts/script_with_arguments/out/debug/script_with_arguments-abi.json"),
            Predicate(name="MyPredicate", abi="packages/fuels/tests/predicates/predicate_basic/out/debug/predicate_basic-abi.json"),
        );
        // ANCHOR_END: multiple_abigen_program_types

        Ok(())
    }
}
