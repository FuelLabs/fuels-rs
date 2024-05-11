contract;

use std::vm::evm::evm_address::EvmAddress;

abi EvmTest {
    fn evm_address_as_input(evm_addr: EvmAddress) -> bool;
    fn evm_address_from_literal() -> EvmAddress;
    fn evm_address_from_argument(raw_address: b256) -> EvmAddress;
}

impl EvmTest for Contract {
    fn evm_address_as_input(evm_addr: EvmAddress) -> bool {
        let evm_addr2 = EvmAddress::from(0x1616060606060606060606060606060606060606060606060606060606060606);

        evm_addr == evm_addr2
    }

    fn evm_address_from_literal() -> EvmAddress {
        EvmAddress::from(0x0606060606060606060606060606060606060606060606060606060606060606)
    }

    fn evm_address_from_argument(raw_address: b256) -> EvmAddress {
        EvmAddress::from(raw_address)
    }
}
