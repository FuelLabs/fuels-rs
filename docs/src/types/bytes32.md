# `Bytes32`

In Sway and the FuelVM, `Bytes32` represents hashes. They hold a 256-bit (32-byte) value. `Bytes32` is a wrapper on a 32-sized slice of `u8`: `pub struct Bytes32([u8; 32]);`.

These are the main ways of creating a `Bytes32`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bytes32}}
```

`Bytes32` also implements the `fmt` module's `Debug`, `Display`, `LowerHex` and `UpperHex` traits. For example, you can get the display and hex representations with:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:bytes32_format}}
```

For a full list of implemented methods and traits, see the [fuel-types documentation](https://docs.rs/fuel-types/{{versions.fuel-types}}/fuel_types/struct.Bytes32.html).

> **Note:** In Fuel, there's a special type called `b256`, which is similar to `Bytes32`; also used to represent hashes, and it holds a 256-bit value. In Rust, through the SDK, this is represented as `Bits256(value)` where `value` is a `[u8; 32]`. If your contract method takes a `b256` as input, all you need to do is pass a `Bits256([u8; 32])` when calling it from the SDK.
