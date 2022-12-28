# Transaction dependency estimation

Previously, we mentioned that a contract call may require you to manually specify external contracts, variable outputs or output messages. The SDK can also attempt to estimate and set these dependencies for you at the cost of running multiple simulated calls in the background.

The following example has a contract call that requires both an external contract and variable outputs. By chaining `.estimate_tx_dependencies()`, the required external contract and the appropriate amount of variable outputs will be set automatically. The optional parameter is the maximum number of simulation attempts:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation}}
```

> **Note:** `.estimate_tx_dependencies()` can also be used when working with multi calls.

> **Note:** if no resolution was found after exhausting all simulation attempts, the last received error will be propagated. The same will happen if an error occurs that is not related to the transactions dependencies.
