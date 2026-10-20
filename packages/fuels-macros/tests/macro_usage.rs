#[cfg(test)]
mod tests {
    #[test]
    fn source_relative_abi() {
        let t = trybuild::TestCases::new();
        t.pass("tests/pass/abigen/*.rs");
    }

    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/abigen/*.rs");
        t.compile_fail("tests/ui/setup_program_test/*.rs");
        t.compile_fail("tests/ui/derive/*/*.rs");
    }
}
