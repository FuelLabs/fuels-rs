#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/abigen_macro/*.rs");
        t.compile_fail("tests/ui/setup_contract_test_macro/*.rs");
    }
}
