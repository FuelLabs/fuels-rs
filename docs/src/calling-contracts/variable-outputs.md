# Output variables

<!-- This section should explain variable outputs  -->
<!-- variable_outputs:example:start -->
Sometimes, the contract you call might transfer funds to a specific address, depending on its execution. The underlying transaction for such a contract call has to have the appropriate number of [variable outputs](https://docs.fuel.network/docs/specs/tx-format/output/#outputvariable) to succeed.
<!-- variable_outputs:example:end -->

Let's say you deployed a contract with the following method:

```rust,ignore
{{#include ../../../e2e/sway/contracts/token_ops/src/main.sw:variable_outputs}}
```

When calling `transfer_coins_to_output` with the SDK, you can specify the number of variable outputs:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:variable_outputs}}
```

<!-- This section should explain what the `with_variable_output_policy` method does -->
<!-- with_variable_output_policy:example:start -->
`with_variable_output_policy` sets the policy regarding variable outputs. You can either set the number of variable outputs yourself by providing `VariableOutputPolicy::Exactly(n)` or let the SDK estimate it for you with `VariableOutputPolicy::EstimateMinimum`. A variable output indicates that the amount and the owner may vary based on transaction execution.
<!-- with_variable_output_policy:example:end -->

> **Note:** that the Sway `lib-std` function `mint_to_address` calls `transfer_to_address` under the hood, so you need to call `with_variable_output_policy` in the Rust SDK tests like you would for `transfer_to_address`.
