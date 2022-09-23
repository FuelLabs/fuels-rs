library wallet_abi;

abi Wallet {
    #[storage(read, write)]
    fn receive_funds();

    #[storage(read, write)]
    fn send_funds(amount_to_send: u64, recipient_address: Address);
}
