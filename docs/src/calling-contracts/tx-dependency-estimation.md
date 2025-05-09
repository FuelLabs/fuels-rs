# Transaction dependency estimation

Previously, we mentioned that a contract call might require you to manually specify external contracts, variable outputs, or output messages. The SDK will estimate and set these dependencies for you.

The following example uses a contract call that calls an external contract and later mints assets to a specified address.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation}}
```
