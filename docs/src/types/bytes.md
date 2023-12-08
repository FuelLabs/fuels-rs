# `Bytes`

In Fuel, a type called `Bytes` represents a collection of tightly-packed bytes. The Rust SDK represents `Bytes` as `Bytes(Vec<u8>)`. Here's an example of using `Bytes` in a contract call:

```rust,ignore
{{#include ../../../packages/fuels/tests/types_contracts.rs:bytes_arg}}
```

If you have a hexadecimal value as a string and wish to convert it to `Bytes`, you may do so with `from_hex_str`:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types/core/bytes.rs:bytes_from_hex_str}}
```
