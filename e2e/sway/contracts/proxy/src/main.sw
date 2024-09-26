contract;

use std::execution::run_external;

abi Proxy {
    #[storage(write)]
    fn set_target_contract(id: ContractId);

    // methods of the `huge_contract` in our e2e sway contracts
    #[storage(read)]
    fn something() -> u64;

    #[storage(read)]
    fn write_some_u64(some: u64);

    #[storage(read)]
    fn read_some_u64() -> u64;
}

storage {
    target_contract: Option<ContractId> = None,
}

impl Proxy for Contract {
    #[storage(write)]
    fn set_target_contract(id: ContractId) {
        storage.target_contract.write(Some(id));
    }

    #[storage(read)]
    fn something() -> u64 {
        let target = storage.target_contract.read().unwrap();
        run_external(target)
    }

    #[storage(read)]
    fn write_some_u64(_some: u64) {
        let target = storage.target_contract.read().unwrap();
        run_external(target)
    }

    #[storage(read)]
    fn read_some_u64() -> u64 {
        let target = storage.target_contract.read().unwrap();
        run_external(target)
    }
}
