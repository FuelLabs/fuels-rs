# Calling other contracts

If your contract method is calling other contracts you will have to add the appropriate `Inputs` and `Outputs` to your transaction. For your convenience, the `CallHandler` will fill in all missing `Inputs`/`Outputs` before sending the transaction.

```rust,ignore
{{#include ../../../e2e/tests/contracts.rs:external_contract}}
```

If you need to decode logs and require revert errors originating from the external contract you will need to pass the `LogDecoder` from the external contract to the contract instance making the call.

```rust,ignore
{{#include ../../../e2e/tests/contracts.rs:external_contract_logs}}
```
