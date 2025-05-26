# Configurable constants

In Sway, you can define `configurable` constants which can be changed during the contract deployment in the SDK. Here is an example how the constants are defined.

```rust,ignore
{{#include ../../../e2e/sway/contracts/configurables/src/main.sw}}
```

Each of the configurable constants will get a dedicated `with` method in the SDK. For example, the constant `STR_4` will get the `with_STR_4` method which accepts the same type as defined in the contract code. Below is an example where we chain several `with` methods and deploy the contract with the new constants.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_configurables}}
```

In addition to writing, you are able to read the configurable constants directly from the binary:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_configurables_reader}}
```

If you need to manually read the configurable constants, you can use helper functions provided by the SDK.

```rust,ignore
{{#include ../../../e2e/tests/configurables.rs:manual_configurables}}
```

Similarly, you can read the configurable constants at runtime.

```rust,ignore
{{#include ../../../e2e/tests/configurables.rs:manual_runtime_configurables}}
```

> **Note:** when manually reading configurable constants make sure to call the appropriate method when dealing with static or dynamic configurables. For dynamic configurables use the `indirect` methods.
