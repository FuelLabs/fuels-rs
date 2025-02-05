#[cfg(test)]
mod tests {
    use fuel_asm::Opcode;
    use fuels::programs::executable::Executable;

    fn test_binary_format(path: &str, expected_jump_offset: u8) {
        let binary =
            std::fs::read(path).unwrap_or_else(|_| panic!("Could not read binary file: {}", path));
        let executable = Executable::from_bytes(binary);
        let loader = executable.convert_to_loader().unwrap();
        let blob = loader.blob();

        let op = Opcode::try_from(blob.as_ref()[4])
            .unwrap_or_else(|_| panic!("Invalid opcode at byte 4 in {}", path));
        let jump_offset = blob.as_ref()[7];

        assert_eq!(op, Opcode::JMPF, "Unexpected opcode at byte 4 in {}", path);
        assert_eq!(
            jump_offset, expected_jump_offset,
            "Unexpected jump offset in {}",
            path
        );
    }

    #[test]
    fn test_legacy_binary_format() {
        test_binary_format(
            "../e2e/assets/precompiled_sway/legacy_format_simple_contract.bin",
            0x02,
        );
    }

    #[test]
    fn test_new_binary_format() {
        test_binary_format(
            "../e2e/sway/bindings/simple_contract/out/release/simple_contract.bin",
            0x04,
        );
    }
}
