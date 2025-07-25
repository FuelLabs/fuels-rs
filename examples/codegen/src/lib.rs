extern crate alloc;

#[cfg(test)]
mod tests {
    #[test]
    fn example_alias() {
        use fuels::code_gen::*;

        let target = AbigenTarget::new(
            "MyContract".into(),
            Abi::load_from("/home/joao/dev/fuels-rs/e2e/sway/abi/contract_with_alias_0/out/release/contract_with_alias-abi.json")
            //Abi::load_from("/home/joao/dev/fuels-rs/e2e/sway/abi/contract_with_alias/out/release/contract_with_alias-abi.json")
            // Abi::load_from("/home/joao/dev/sway/_test_aliases_abi/out/debug/test-case-abi.json")
                .unwrap(),
            ProgramType::Contract,
        );
        let targets = vec![target];

        let abigen = Abigen::generate(targets, false).expect("abigen generation failed");
    }
}
