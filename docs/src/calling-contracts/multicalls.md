# Multiple contract calls

With `CallHandler`, you can execute multiple contract calls within a single transaction. To achieve this, you first prepare all the contract calls that you want to bundle:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_prepare}}
```

You can also set call parameters, variable outputs, or external contracts for every contract call, as long as you don't execute it with `call()` or `simulate()`.

> **Note:** if custom inputs or outputs have been added to the separate calls, the input and output order will follow the order how the calls are added to the multi-call.

Next, you provide the prepared calls to your `CallHandler` and optionally configure transaction policies:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_build}}
```

> **Note:** any transaction policies configured on separate contract calls are disregarded in favor of the parameters provided to the multi-call `CallHandler`.

Furthermore, if you need to separate submission from value retrieval for any reason, you can do so as follows:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:submit_response_multicontract}}
```

## Output values

To get the output values of the bundled calls, you need to provide explicit type annotations when saving the result of `call()` or `simulate()` to a variable:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_values}}
```

You can also interact with the `CallResponse` by moving the type annotation to the invoked method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_contract_call_response}}
```
