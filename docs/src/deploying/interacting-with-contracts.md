# Interacting with contracts

If you already have a deployed contract and want to call its methods using the SDK,  but without deploying it again, all you need is the contract ID of your deployed contract. You can skip the whole deployment setup and call `::new(contract_id, wallet)` directly. For example:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deployed_contracts}}
```

The above example assumes that your contract ID string is encoded in the `bech32` format. You can recognize it by the human-readable-part "fuel" followed by the separator "1". However, when using other Fuel tools, you might end up with a hex-encoded contract ID string. In that case, you can create your contract instance as follows:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:deployed_contracts_hex}}
```

You can learn more about the Fuel SDK `bech32` types [here](../types/bech32.md).
