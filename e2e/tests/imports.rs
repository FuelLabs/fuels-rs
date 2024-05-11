#[cfg(test)]
mod tests {
    #[test]
    fn provides_output_type() {
        // test exists because we've excluded fuel_tx::Output twice
        #[allow(unused_imports)]
        use fuels::types::output::Output;
    }
}
