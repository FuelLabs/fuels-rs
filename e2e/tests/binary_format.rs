#[cfg(test)]
mod tests {
    use fuels::programs::executable::{Executable, Regular};
    use std::convert::TryInto;
    use std::ops::Range;

    const DATA_OFFSET_LOCATION: Range<usize> = 8..16;
    const CONFIGURABLES_OFFSET_LOCATION: Range<usize> = 16..24;

    const LEGACY_BINARY_PATH: &str =
        "../e2e/assets/precompiled_sway/legacy_format_simple_contract.bin";
    const NEW_BINARY_PATH: &str =
        "../e2e/sway/bindings/simple_contract/out/release/simple_contract.bin";

    #[test]
    fn no_configurables_offset_for_old_sway_binaries() {
        // given
        let (_, executable) = load(LEGACY_BINARY_PATH);

        // when
        let configurables_offset = executable.configurables_offset_in_code().unwrap();

        // then
        assert_eq!(configurables_offset, None);
    }

    #[test]
    fn correct_data_offset_for_old_sway_binaries() {
        // given
        let (binary, executable) = load(LEGACY_BINARY_PATH);
        let expected_data_offset = read_offset(&binary, DATA_OFFSET_LOCATION);

        // when
        let data_offset = executable.data_offset_in_code().unwrap();

        // then
        assert_eq!(data_offset, expected_data_offset);
    }

    #[test]
    fn correct_data_offset_for_new_sway_binaries() {
        // given
        let (binary, executable) = load(NEW_BINARY_PATH);
        let expected_data_offset = read_offset(&binary, DATA_OFFSET_LOCATION);

        // when
        let data_offset = executable.data_offset_in_code().unwrap();

        // then
        assert_eq!(data_offset, expected_data_offset);
    }

    #[test]
    fn correct_configurables_offset_for_new_sway_binaries() {
        // given
        let (binary, executable) = load(NEW_BINARY_PATH);
        let expected_configurables_offset = read_offset(&binary, CONFIGURABLES_OFFSET_LOCATION);

        // when
        let configurables_offset = executable.configurables_offset_in_code();

        // then
        let configurables_offset = configurables_offset
            .expect("to successfully detect a modern binary is used")
            .expect("to extract the configurables_offset");
        assert_eq!(configurables_offset, expected_configurables_offset);
    }

    pub fn read_offset(binary: &[u8], range: Range<usize>) -> usize {
        assert_eq!(range.clone().count(), 8, "must be a range of 8 B");
        let data: [u8; 8] = binary[range].try_into().unwrap();
        u64::from_be_bytes(data) as usize
    }

    fn load(path: &str) -> (Vec<u8>, Executable<Regular>) {
        let binary = std::fs::read(path).unwrap();
        let executable = Executable::from_bytes(binary.clone());
        (binary, executable)
    }
}
