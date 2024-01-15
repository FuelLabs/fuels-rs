# Signing

Once you've instantiated your wallet in an unlocked state using one of the previously discussed methods, you can sign a message with `wallet.sign`. Below is a full example of how to sign and recover a message.

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/account.rs:sign_message}}
```

## Adding `Signers` to a transaction builder

Every signed resource in the inputs needs to have a witness index that points to a valid witness. Changing the witness index inside an input will change the transaction ID. This means that we need to set all witness indexes before finally signing the transaction. Previously, the user had to make sure that the witness indexes and the order of the witnesses are correct. To automate this process, the SDK will keep track of the signers in the transaction builder and resolve the final transaction automatically. This is done by storing signers until the final transaction is built.

Below is a full example of how to create a transaction builder and add signers to it.

> Note: When you add a `Signer` to a transaction builder, the signer is stored inside it and the transaction will not be resolved until you call `build()`!

```rust,ignore
{{#include ../../../packages/fuels-accounts/src/account.rs:sign_tb}}
```

## Signing a built transaction

If you have a built transaction and want to add a signature, you can use the `sign_with` method.

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts.rs:tx_sign_with}}
```
