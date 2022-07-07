# Calling other contracts

Your contract method might be calling other contracts. To do so, you must feed the external contract IDs that your contract depends on to the method you're calling. You do it by chaining `.set_contracts(&[external_contract_id, ...])` to the method you want to call. For instance:

```rust,ignore
{{#include ../../../packages/fuels-abigen-macro/tests/harness.rs:external_contract}}
```
