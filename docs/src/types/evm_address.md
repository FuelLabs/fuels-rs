# EvmAddress

In the Rust SDK, evm addresses can be represented with the 'EvmAddress' type. Its definition matches with the Sway standard library type with the same name and will be converted accordingly when interacting with contracts:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types.rs:evm_address}}
```

Here's an example:

```rust,ignore
{{#include ../../../packages/fuels/tests/bindings.rs:evm_address_arg}}
```
