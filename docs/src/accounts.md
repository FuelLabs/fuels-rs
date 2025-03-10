# Accounts

The `ViewOnlyAccount` trait provides a common interface to query balances.

The `Account` trait, in addition to the above, also provides a way to transfer assets. When performing actions in the SDK that lead to a transaction, you will typically need to provide an account that will be used to allocate resources required by the transaction, including transaction fees.

The traits are implemented by the following types:

- [`Wallet`](./wallets/index.md)
- [`Predicate`](./predicates/index.md)

## Transferring assets

An account implements the following methods for transferring assets:

- `transfer`
- `force_transfer_to_contract`
- `withdraw_to_base_layer`

The following examples are provided for a `Wallet` account. A `Predicate` account would work similarly, but you might need to set its predicate data before attempting to spend resources owned by it.

With `wallet.transfer` you can initiate a transaction to transfer an asset from your account to a target address.

```rust,ignore
{{#include ../../examples/wallets/src/lib.rs:wallet_transfer}}
```

You can transfer assets to a contract via `wallet.force_transfer_to_contract`.

```rust,ignore
{{#include ../../examples/wallets/src/lib.rs:wallet_contract_transfer}}
```

For transferring assets to the base layer chain, you can use `wallet.withdraw_to_base_layer`.

```rust,ignore
{{#include ../../examples/wallets/src/lib.rs:wallet_withdraw_to_base}}
```

The above example creates an `Address` from a string and converts it to a `Bech32Address`. Next, it calls `wallet.withdraw_to_base_layer` by providing the address, the amount to be transferred, and the transaction policies. Lastly, to verify that the transfer succeeded, the relevant message proof is retrieved with `provider.get_message_proof,` and the amount and the recipient are verified.
