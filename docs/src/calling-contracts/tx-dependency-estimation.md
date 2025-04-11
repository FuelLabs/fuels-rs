# Transaction dependency estimation

Previously, we mentioned that a contract call might require you to manually specify external contracts, variable outputs, or output messages. The SDK can also attempt to estimate and set these dependencies for you at the cost of running multiple simulated calls in the background.

The following example uses a contract call that calls an external contract and later mints assets to a specified address. Calling it without including the dependencies will result in a revert:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation_fail}}
```

As mentioned in previous chapters, you can specify the external contract and add an output variable to resolve this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation_manual}}
```

But this requires you to know the contract ID of the external contract and the needed number of output variables. Alternatively, by chaining

- `.with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)` and
- `.determine_missing_contracts()`

the dependencies will be estimated by the SDK and set automatically.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation}}
```

> **Note:** Both `with_variable_output_policy` and `determine_missing_contracts` can also be used when working with script calls or multi calls. `determine_missing_contracts()` will not enable logging from an external contract. For more information, see [here](./other-contracts.md).
