contract;

abi MyContract {
    fn something() -> u64;
    #[storage(write)]
    fn write_some_u64(some: u64);
    #[storage(read)]
    fn read_some_u64() -> u64;
}

storage {
    some_u64: u64 = 42,
}

impl MyContract for Contract {
    fn something() -> u64 {
        asm() {
            blob i450000;
        }
        1001
    }

    #[storage(write)]
    fn write_some_u64(some: u64) {
        storage.some_u64.write(some);
    }

    #[storage(read)]
    fn read_some_u64() -> u64 {
        storage.some_u64.read()
    }
}
