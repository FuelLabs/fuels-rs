# Customizing contract and script calls

When preparing a contract call via `ContractCallHandler` or a script call via `ScriptCallHandler`, the Rust SDK uses a transaction builder in the background. You can fetch this builder and customize it before submitting it to the network. After the transaction is executed successfully, you can use the corresponding `ContractCallHandler` or `ScriptCallHandler` to generate a `FuelCallResponse`.

## Custom contract calls

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_receiver}}
```

## Custom script calls

```rust,ignore
{{#include ../../../examples/cookbook/src/lib.rs:custom_tx_receiver}}
```
