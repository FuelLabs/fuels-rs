# abigen

<!-- This section explain the `abigen!` macro -->
<!-- abigen:example:start -->
`abigen!` is a procedural macro -- it generates code. It accepts inputs in the format of:

```text
ProgramType(name="MyProgramType", abi="my_program-abi.json")...
```

where:

- `ProgramType` is one of: `Contract`, `Script` or `Predicate`,

- `name` is the name that will be given to the generated bindings,

- `abi` is either a path to the json abi file or its actual contents.
<!-- abigen:example:end -->

---
So, an `abigen!` which generates bindings for two contracts and one script looks like this:

```rust,ignore
{{#include ../../../examples/macros/src/lib.rs:multiple_abigen_program_types}}
```

## How does the generated code look?

A rough overview:

```rust,ignore
pub mod abigen_bindings {
    pub mod contract_a_mod {
        struct SomeCustomStruct{/*...*/};
        // other custom types used in the contract

        struct ContractA {/*...*/};
        impl ContractA {/*...*/};
        // ...
    }
    pub mod contract_b_mod {
        // ...
    }
    pub mod my_script_mod {
        // ...
    }
    pub mod my_predicate_mod{
        // ...
    }
    pub mod shared_types{
        // ...
    }
}

pub use contract_a_mod::{/*..*/};
pub use contract_b_mod::{/*..*/};
pub use my_predicate_mod::{/*..*/};
pub use shared_types::{/*..*/};
```

Each `ProgramType` gets its own `mod` based on the `name` given in the `abigen!`. Inside the respective mods, the custom types used by that program are generated, and the bindings through which the actual calls can be made.

One extra `mod` called `shared_types` is generated if `abigen!` detects that the given programs share types. Instead of each `mod` regenerating the type for itself, the type is lifted out into the `shared_types` module, generated only once, and then shared between all program bindings that use it. Reexports are added to each mod so that even if a type is deemed shared, you can still access it as though each `mod` had generated the type for itself (i.e. `my_contract_mod::SharedType`).

A type is deemed shared if its name and definition match up. This can happen either because you've used the same library (a custom one or a type from the stdlib) or because you've happened to define the exact same type.

Finally, `pub use` statements are inserted, so you don't have to fully qualify the generated types. To avoid conflict, only types that have unique names will get a `pub use` statement. If you find rustc can't find your type, it might just be that there is another generated type with the same name. To fix the issue just qualify the path by doing `abigen_bindings::whatever_contract_mod::TheType`.

> **Note:**
> It is **highly** encouraged that you generate all your bindings in one `abigen!` call. Doing it in this manner will allow type sharing and avoid name collisions you'd normally get when calling `abigen!` multiple times inside the same namespace. If you choose to proceed otherwise, keep in mind the generated code overview presented above and appropriately separate the `abigen!` calls into different modules to resolve the collision.

### Type paths

Normally when using types from libraries in your contract, script or predicate, they'll be generated directly under the main `mod` of your program bindings, i.e. a type in a contract binding `MyContract` imported from a library `some_library` would be generated under `abigen_bindings::my_contract_mod::SomeLibraryType`.

This can cause problems if you happen to have two types with the same name in different libraries of your program.

This behavior can be changed to include the library path by compiling your Sway project with the following:

```shell
forc build --json-abi-with-callpaths
```

Now the type from the previous example will be generated under `abigen_bindings::my_contract_mod::some_library::SomeLibraryType`.

This might only become relevant if your type isn't reexported. This can happen, as explained previously, if your type does not have a unique name across all bindings inside one `abigen!` call. You'll then need to fully qualify the access to it.

Including type paths will eventually become the default and the flag will be removed.

## Using the bindings

Let's look at a contract with two methods: `initialize_counter(arg: u64) -> u64` and `increment_counter(arg: u64) -> u64`, with the following JSON ABI:

```json,ignore
{{#include ../../../examples/rust_bindings/src/abi.json}}
```

By doing this:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:use_abigen}}
```

or this:

```rust,ignore
{{#include ../../../examples/rust_bindings/src/lib.rs:abigen_with_string}}
```

you'll generate this (shortened for brevity's sake):

```rust,ignore
{{#include ../../../examples/rust_bindings/src/rust_bindings_formatted.rs}}
```

> **Note:** that is all **generated** code. No need to write any of that. Ever. The generated code might look different from one version to another, this is just an example to give you an idea of what it looks like.

Then, you're able to use it to call the actual methods on the deployed contract:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:use_deployed_contract}}
```
