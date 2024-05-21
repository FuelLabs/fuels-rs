# `ContractId`

Like `Bytes32`, `ContractId` is a wrapper on `[u8; 32]` with similar methods and implements the same traits (see [fuel-types documentation](https://docs.rs/fuel-types/0.49.0/fuel_types/struct.ContractId.html)).

These are the main ways of creating a `ContractId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:contract_id}}
```
