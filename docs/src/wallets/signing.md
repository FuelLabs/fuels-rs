# Signing

Once you've instantiated your wallet in an unlocked state using one of the previously discussed methods, you can sign a message with `wallet.sign_message`. Below is a full example of how to sign and recover a message.

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/lib.rs:sign_message}}
```

## Signing a transaction

- what it means and how it is done - every signed resource needs to have a witness index and witness
- what we do for the user - save the signing intention, resolve all indexes and signatures
- add comments in code

You can also sign a _transaction builder_ by using `wallet.sign_transaction`. Below is a full example of how to sign and recover a transaction.

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/lib.rs:sign_tx}}
```
