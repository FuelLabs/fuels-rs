# Customizing contract and script calls

When preparing a contract call via `ContractCallHandler` or a script call via `ScriptCallHandler`, the Rust SDK uses a transaction builder in the background. You can fetch this builder and customize it before submitting it to the network. After the transaction is executed successfully, you can use the corresponding `ContractCallHandler` or `ScriptCallHandler` to generate a [call response](../calling-contracts/call-response.md). Below are examples for each use case.

## Custom contract call

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_call_tb}}
```

## Custom script call

```rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_call_tb}}
```
