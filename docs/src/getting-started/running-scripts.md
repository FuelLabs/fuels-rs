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
Scripts use the same interfaces for setting external contracts as [contract methods](../calling-contracts/other-contracts.md).

Below is an example that uses `set_contracts(&[&contract_instance, ...])`.

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:external_contract}}
```
And this is an example that uses `set_contract_ids(&[&contract_id, ...])`.

```rust,ignore
{{#include ../../../packages/fuels/tests/logs.rs:external_contract_ids}}
```
