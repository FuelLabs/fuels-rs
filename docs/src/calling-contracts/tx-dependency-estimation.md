# Transaction dependency estimation

Previously, we mentioned that a contract call might require you to manually specify external contracts, variable outputs, or output messages. The SDK can also attempt to estimate and set these dependencies for you at the cost of running multiple simulated calls in the background.

The following example uses a contract call that calls an external contract and later mints assets to a specified address. Calling it without including the dependencies will result in a revert:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation_fail}}
```

As mentioned in previous chapters, you can specify the external contract with `.set_contracts()` and add an output variable with `append_variable_outputs()` to resolve this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation_manual}}
```

But this requires you to know the contract id of the external contract and the needed number of output variables. Alternatively, by chaining `.estimate_tx_dependencies()` instead, the dependencies will be estimated by the SDK and set automatically. The optional parameter is the maximum number of simulation attempts:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:dependency_estimation}}
```

The minimal number of attempts corresponds to the number of external contracts and output variables needed and defaults to 10.

> **Note:** `estimate_tx_dependencies()` can also be used when working with multi calls.

> **Note:** `estimate_tx_dependencies()` does not currently resolve the dependencies needed for logging from an external contract. For more information, see [here](./calling-contracts/logs.md).

> **Note:** if no resolution was found after exhausting all simulation attempts, the last received error will be propagated. The same will happen if an error is unrelated to transaction dependencies.
