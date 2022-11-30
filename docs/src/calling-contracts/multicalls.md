# Multiple contract calls

With `ContractMultiCallHandler`, you can execute multiple contract calls within a single transaction. To achieve this, you first prepare all the contract calls that you want to bundle:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_prepare}}
```

You can also set call parameters, variable outputs, or external contracts for every contract call, as long as you don't execute it with `call()` or `simulate()`.

Next, you provide the prepared calls to your `ContractMultiCallHandler` and optionally configure transaction parameters:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_build}}
```

> **Note:** any transaction parameters configured on separate contract calls are disregarded in favor of the parameters provided to `ContractMultiCallHandler`.

## Output values

To get the output values of the bundled calls, you need to provide explicit type annotations when saving the result of `call()` or `simulate()` to a variable:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_values}}
```

You can also interact with the `FuelCallResponse` by moving the type annotation to the invoked method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_contract_call_response}}
```
