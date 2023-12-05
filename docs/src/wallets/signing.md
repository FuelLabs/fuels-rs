# Signing

Once you've instantiated your wallet in an unlocked state using one of the previously discussed methods, you can sign a message with `wallet.sign_message`. Below is a full example of how to sign and recover a message.

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/account.rs:sign_message}}
```

## Signing a transaction

Every signed resource in the inputs needs to have a witness index that points to a valid witness. Changing the witness index inside an input will change the transaction ID. This means that we need to set all witness indexes before finally signing the transaction. Previously, the user had to make sure that the witness indexes and the order of the witnesses are correct. To automate this process, the SDK will keep track of the signatures in the transaction builder and resolve the final transaction automatically. This is done by storing the secret keys of all signers until the final transaction is built.

To sign a _transaction builder_ use the `wallet.sign_transaction`. Below is a full example of how to create a transaction and sign it.

> Note: When you sign a transaction builder the secret key is stored inside it and will not be resolved until you call `build()`!

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/account.rs:sign_tx}}
```
