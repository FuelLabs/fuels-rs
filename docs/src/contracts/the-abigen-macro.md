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

And this `abigen!` macro will _expand_ the code with the type-safe Rust bindings. It accepts input in the form of:


`ProgramType(name = "SomeName", abi="some-abi.json")`,

where:

`ProgramType` is either `Contract`, `Script` or `Predicate`,

`name = "SomeName"` is the name of the generated struct inside the generated mod `some_name_mod`,

`abi = "some-abi.json"` is the JSON file containing the ABI from which the bindings are to be generated.


The same as the example above but passing the ABI definition directly:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:abigen_with_string}}
```

## Generating multiple bindings at once
If your contracts, scripts or predicates share types via libraries, you should consider generating the bindings for all
the programs at once:

This way, types that are equal in both name and contents will be extracted into a separate mod called `shared_types`.
This way you can seamlessly use them between the various generated bindings.

Otherwise, if you generate every binding separately, every binding is going to have its own type, and you'll need to
convert between them.

### Known caveat of shared-types
If shared_types are being generated then you cannot call `abigen!` inside a function. This is due to the bindings
internally referring to the shared types via `super::shared_types::SomeSharedType` which doesn't work if `super` refers
to the crate root while the bindings are in a function.

A workaround would be to wrap everything inside another mod so that `super::` refers to it instead of the crate root:
