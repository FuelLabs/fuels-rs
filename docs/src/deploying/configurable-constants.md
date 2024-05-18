# Configurable constants

In Sway, you can define `configurable` constants which can be changed during the contract deployment in the SDK. Here is an example how the constants are defined.

```rust,ignore
{{#include ../../../e2e/sway/contracts/configurables/src/main.sw}}
```

Each of the configurable constants will get a dedicated `with` method in the SDK. For example, the constant `STR_4` will get the `with_STR_4` method which accepts the same type as defined in the contract code. Below is an example where we chain several `with` methods and deploy the contract with the new constants.

```rust,ignore
{{#include ../../../e2e/tests/configurables.rs:contract_configurables}}
```
