contract;

use std::address::Address;
use std::storage::store;
use std::storage::get;

const OWNER_ADDRESS = ~Address::from(0x8900c5bec4ca97d4febf9ceb4754a60d782abbf3cd815836c1872116f203f861);

storage {
    balance: u64 = 0,
}

abi Wallet {
    #[storage(read, write)]
    fn receive_funds();

    #[storage(read, write)]
    fn send_funds(amount_to_send: u64, recipient_address: Address);
}

impl Wallet for Contract {
    #[storage(read, write)]
    fn receive_funds() {
        if msg_asset_id() == BASE_ASSET_ID {
            storage.balance += msg_amount();
        }
    }

    #[storage(read, write)]
    fn send_funds(amount_to_send: u64, recipient_address: Address) {
        let sender: Result<Identity, AuthError> = msg_sender();
        match sender.unwrap() {
            Identity::Address(addr) => assert(addr == OWNER_ADDRESS),
            _ => revert(0),
        };

        let current_balance = storage.balance;
        assert(current_balance >= amount_to_send);

        storage.balance = current_balance - amount_to_send;

        // Note: `transfer_to_output()` is not a call and thus not an
        // interaction. Regardless, this code conforms to
        // checks-effects-interactions to avoid re-entrancy.
        transfer_to_output(amount_to_send, BASE_ASSET_ID, recipient_address);
    }
}
