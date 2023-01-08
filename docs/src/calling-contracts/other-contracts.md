# Calling other contracts

If your contract method is calling other contracts you will have to add the appropriate `Inputs` and `Outputs` to your transaction. For your convenience, the `ContractCallHandler` provides methods that prepare those inputs and outpus for you. You have two methods that you can use: `set_contracts(&[&contract_instance, ...])` and `set_contract_ids(&[&contract_id, ...])`.

`set_contracts(&[&contract_instance, ...])` requires contract instances that were created using the `abigen` macro. When setting the external contracts with this method, logs and require revert errors originating from the external contract can be propagated and decoded by the calling contract.

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts.rs:external_contract}}
```

 If however, you do not need do decode logs or you do not have a contract instance that was generated using the `abigen` macro you can use `set_contract_ids(&[&contract_id, ...])` and provide the required contract ids.

```rust,ignore
{{#include ../../../packages/fuels/tests/contracts.rs:external_contract_ids}}
```
