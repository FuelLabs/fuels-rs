#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/abigen/*.rs");
        t.compile_fail("tests/ui/setup_program_test/*.rs");
        t.compile_fail("tests/ui/derive/*/*.rs");
    }
}
