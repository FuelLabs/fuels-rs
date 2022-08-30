# Calls with different wallets

You can use the `_with_wallet()` method on an existing contract instance as a shorthand for creating a new instance connected to the provided wallet. This lets you make contracts calls with different wallets in a chain like fashion.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:connect_wallet}}
```

> **Note:** connecting a different wallet to an existing instance ignores its set provider in favor of the provider used to deploy the contract. If you have two wallets connected to separate providers (each communicating with a separate fuel-core), the one assigned to the deploying wallet will also be used for contract calls. This behavior is only relevant if multiple providers (i.e. fuel-core instances) are present and can otherwise be ignored.