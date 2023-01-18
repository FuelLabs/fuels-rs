# EvmAddress

In the Rust SDK, Ethereum Virtual Machine (EVM) addresses can be represented with the 'EvmAddress' type. Its definition matches with the Sway standard library type with the same name and will be converted accordingly when interacting with contracts:

```rust,ignore
{{#include ../../../packages/fuels-types/src/core/bits.rs:evm_address}}
```

Here's an example:

```rust,ignore
{{#include ../../../packages/fuels/tests/bindings.rs:evm_address_arg}}
```

> **Note:** when creating an `EvmAddress` from `Bits256`, the first 12 bytes will be cleared because an evm address is only 20 bytes long.
