# Generating bindings with abigen

You might have noticed this snippet in the previous sections:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:abigen_example}}
```

<!-- This section should explain the purpose of the abigen -->
<!-- abigen:example:start -->
The SDK lets you transform ABI methods of a smart contract, specified as JSON objects (which you can get from [Forc](https://github.com/FuelLabs/sway/tree/master/forc)), into Rust structs and methods that are type-checked at compile time.
In order to call your contracts, scripts or predicates, you first need to generate the Rust bindings for them.
<!-- abigen:example:end -->

The following subsections contain more details about the `abigen!` syntax and the code generated from it.
