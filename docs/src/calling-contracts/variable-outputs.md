# Variable outputs

In some cases, you might want to send funds to the output of a transaction. Sway has a specific method for that: `transfer_to_address`(coins, asset_id, recipient)`. So, if you have a contract that does something like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts/token_ops/src/main.sw:variable_outputs}}
```

With the SDK, you can call `transfer_coins_to_output` by chaining `append_variable_outputs(amount)` to your contract call. Like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:variable_outputs}}
```

`append_variable_outputs` effectively appends a given amount of `Output::Variable`s to the transaction's list of outputs. This output type indicates that the output's amount and the owner may vary based on transaction execution.

Note that the Sway `lib-std` function `mint_to_address` calls `transfer_to_address` under the hood, so you need to call `append_variable_outputs` in the Rust SDK tests like you would for `transfer_to_address`.
