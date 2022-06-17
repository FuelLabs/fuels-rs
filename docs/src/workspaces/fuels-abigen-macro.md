# `fuels-abigen-macro`

`fuels-rs`'s abigen is a procedural macro used to transform a contract's ABI defined as a JSON object into type-safe Rust bindings, i.e. Rust structs and types that represent that contract's ABI. These bindings are then expanded and brought into scope.

The specifications for the JSON ABI format and its encoding/decoding can be found [here](https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/abi.md#json-abi-format).

## Usage

A simple example of generating type-safe bindings from a JSON ABI specified in-line:

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:bindings_from_inline_contracts}}
```

This example and many more can be found under `tests/harness.rs`. To run the whole test suite run `cargo test` inside `fuels-abi-gen-macro/`.
