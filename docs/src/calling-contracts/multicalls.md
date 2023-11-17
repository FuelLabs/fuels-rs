# Multiple contract calls

With `MultiContractCallHandler`, you can execute multiple contract calls within a single transaction. To achieve this, you first prepare all the contract calls that you want to bundle:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_prepare}}
```

You can also set call parameters, variable outputs, or external contracts for every contract call, as long as you don't execute it with `call()` or `simulate()`.

Next, you provide the prepared calls to your `MultiContractCallHandler` and optionally configure transaction policies:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_build}}
```

> **Note:** any transaction policies configured on separate contract calls are disregarded in favor of the parameters provided to `MultiContractCallHandler`.

Furthermore, if you need to separate submission from value retrieval for any reason, you can do so as follows:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:submit_response_multicontract}}
```

## Output values

To get the output values of the bundled calls, you need to provide explicit type annotations when saving the result of `call()` or `simulate()` to a variable:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_call_values}}
```

You can also interact with the `FuelCallResponse` by moving the type annotation to the invoked method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:multi_contract_call_response}}
```

> **Note:** The `MultiContractCallHandler` supports only one contract call that returns a heap type. Because of the way heap types are handled, this contract call needs to be at the last position, i.e., added last with `add_call`. This is a temporary limitation that we hope to lift soon. In the meantime, if you have multiple calls handling heap types, split them across multiple regular, single calls.
