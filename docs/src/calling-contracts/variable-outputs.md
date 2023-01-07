# Output variables

Sometimes, the contract you call might transfer funds to a specific address, depending on its execution. The underlying transaction for such a contract call has to have the appropriate number of [variable outputs](https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/output.md#outputvariable) to succeed.

Let's say you deployed a contract with the following method:

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts/token_ops/src/main.sw:variable_outputs}}
```

When calling `transfer_coins_to_output` with the SDK, you can specify the number of variable outputs by chaining `append_variable_outputs(amount)` to your call. Like this:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:variable_outputs}}
```

`append_variable_outputs` effectively appends a given amount of `Output::Variable`s to the transaction's list of outputs. This output type indicates that the amount and the owner may vary based on transaction execution.

> **Note:** that the Sway `lib-std` function `mint_to_address` calls `transfer_to_address` under the hood, so you need to call `append_variable_outputs` in the Rust SDK tests like you would for `transfer_to_address`.

# Output messages

Similarly, when your contract transfers messages, the underlying transaction has to have the appropriate number of [output messages](https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/output.md#outputmessage). 

Output messages can be added to a contract call by chaining `append_output_messages(amount)`:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:message_outputs}}
```
