# The abigen! macro

You might have noticed this section in the previous example:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:abigen_example}}
```

The SDK lets you transform ABI methods of a smart contract, specified as JSON objects (which you can get from [Forc](https://github.com/FuelLabs/sway/tree/master/forc)), into Rust structs and methods that are type-checked at compile time.

For instance, a contract with two methods: `initialize_counter(arg: u64) -> u64` and `increment_counter(arg: u64) -> u64`, with the following JSON ABI:

```json,ignore
{{#include ../../../examples/rust_bindings/src/abi.json}}
```

Can become this (shortened for brevity's sake):

```rust,ignore
{{#include ../../../examples/rust_bindings/src/rust_bindings_formatted.rs}}
```

> **Note:** that is all **generated** code. No need to write any of that. Ever. The generated code might look different from one version to another, this is just an example to give you an idea of what it looks like.

Then, you're able to use it to call the actual methods on the deployed contract:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```

To generate these bindings, all you have to do is:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:use_abigen}}
```

And this `abigen!` macro will _expand_ the code with the type-safe Rust bindings. It takes two arguments:

1. The name of the struct that will be generated (`MyContractName`);
2. Either a path as a string to the JSON ABI file or the JSON ABI as a multiline string directly.

The same as the example above but passing the ABI definition directly:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:abigen_with_string}}
```

## Manual decoding

@todo this must be moved to types section.

Suppose you wish to decode raw bytes into a type used in your contract and the `abigen!` generated this type, then you can use `try_into`:

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:manual_decode}}
```

Otherwise, for native types such as `u8`, `u32`,...,`ContractId` and others, you must use `::fuels::core::try_from_bytes`:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:manual_decode_native}}
```
