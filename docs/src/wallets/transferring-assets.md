# Transferring assets

With `wallet.transfer` you can initiate a transaction to transfer an asset from your wallet to a target address.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:wallet_transfer}}
```

You can also transfer assets to a contract via `wallet.force_transfer_to_contract`.

```rust,ignore
{{#include ../../../examples/wallets/src/lib.rs:wallet_contract_transfer}}
```
