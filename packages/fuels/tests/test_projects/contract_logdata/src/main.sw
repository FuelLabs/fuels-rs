contract;

abi MyContract {
    fn use_logd_opcode(value: b256, len1: u8, len2: u8) -> u64;
    fn dont_use_logd() -> u64;
}

impl MyContract for Contract {
    fn use_logd_opcode(value: b256, len1: u8, len2: u8) -> u64 {
        asm(r1: value, r2: len1, r3: len2, r4: 22, r5: 11) {
            // Log $r2 bytes of value `value`
            logd r4 r5 r1 r2;
            // Log $r3 bytes of value `value`
            logd r5 r4 r1 r3;
        };
        42
    }
    fn dont_use_logd() -> u64 {
        24
    }
}
