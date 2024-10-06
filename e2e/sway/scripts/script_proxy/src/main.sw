script;

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

fn main(proxy_contract_id: ContractId) -> bool {
    let proxy_instance = abi(Proxy, proxy_contract_id.into());
    let _ = proxy_instance.something();
    proxy_instance.write_some_u64(10001);
    let read_u_64 = proxy_instance.read_some_u64();
    return read_u_64 == 10001;
}
