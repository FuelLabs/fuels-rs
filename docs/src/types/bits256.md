# `Bits256`

In Fuel, a type called `b256` represents hashes and holds a 256-bit value. The Rust SDK represents `b256` as `Bits256(value)` where `value` is a `[u8; 32]`. If your contract method takes a `b256` as input, you must pass a `Bits256([u8; 32])` when calling it from the SDK.

Here's an example:

```rust,ignore
{{#include ../../../packages/fuels/tests/bindings.rs:256_arg}}
```

If you have a hexadecimal value as a string and wish to convert it to `Bits256`, you may do so with `from_hex_str`:

```rust,ignore
{{#include ../../../packages/fuels-core/src/types/core/bits.rs:from_hex_str}}
```
