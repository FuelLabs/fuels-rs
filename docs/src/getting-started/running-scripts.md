# Running scripts

You can run a script using its JSON-ABI and the path to its binary file. You can run the scripts with arguments. For this, you have to use the `script_abigen!` macro, which is similar to the `abigen!` macro seen [previously](../contracts/the-abigen-macro.md).

````rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_arguments}}
````

## Running scripts with transaction parameters

The method for passing transaction parameters is the same as [with contracts](../calling-contracts/tx-params.md). As a reminder, the workflow would look like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_tx_params}}
```

## Logs

Script calls provide the same logging functions, `get_logs()` and `get_logs_with_type<T>()`, as contract calls. As a reminder, the workflow looks like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:script_logs}}
```

## Calling contracts from scripts
Scripts can use the same interfaces for setting external contracts as [contract methods](../calling-contracts/other-contracts.md).

As a reminder, `set_contracts(&[&contract_instance, ...])` requires contract instances that were created using the `abigen` macro. When setting the external contracts with this method, logs and require revert errors originating from the external contract can be propagated and decoded by the calling contract.

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:external_contract}}
```
 If however, you do not need do decode logs or you do not have a contract instance that was generated using the `abigen` macro you can use `set_contract_ids(&[&contract_id, ...])` and provide the required contract ids.

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:external_contract_ids}}
```
