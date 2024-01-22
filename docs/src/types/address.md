# `Address`

Like `Bytes32`, `Address` is a wrapper on `[u8; 32]` with similar methods and implements the same traits (see [fuel-types documentation](https://docs.rs/fuel-types/latest/fuel_types/struct.Address.html)).

These are the main ways of creating an `Address`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:address}}
```
