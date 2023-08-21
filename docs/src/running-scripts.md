# Running scripts

You can run a script using its JSON-ABI and the path to its binary file. You can run the scripts with arguments. For this, you have to use the `abigen!` macro seen [previously](./abigen/the-abigen-macro.md).

````rust,ignore
{{#include ../../packages/fuels/tests/scripts.rs:script_with_arguments}}
````

## Running scripts with transaction parameters

The method for passing transaction parameters is the same as [with contracts](./calling-contracts/tx-params.md). As a reminder, the workflow would look like this:

```rust,ignore
{{#include ../../packages/fuels/tests/scripts.rs:script_with_tx_params}}
```

## Logs

Script calls provide the same logging functions, `decode_logs()` and `decode_logs_with_type<T>()`, as contract calls. As a reminder, the workflow looks like this:

```rust,ignore
{{#include ../../packages/fuels/tests/logs.rs:script_logs}}
```

## Calling contracts from scripts

Scripts use the same interfaces for setting external contracts as [contract methods](./calling-contracts/other-contracts.md).

Below is an example that uses `set_contracts(&[&contract_instance, ...])`.

```rust,ignore
{{#include ../../packages/fuels/tests/logs.rs:external_contract}}
```

And this is an example that uses `set_contract_ids(&[&contract_id, ...])`.

```rust,ignore
{{#include ../../packages/fuels/tests/logs.rs:external_contract_ids}}
```

## Configurable constants

Same as contracts, you can define `configurable` constants in `scripts` which can be changed during the script execution. Here is an example how the constants are defined.

```rust,ignore
{{#include ../../packages/fuels/tests/scripts/script_configurables/src/main.sw}}
```

Each configurable constant will get a dedicated `set` method in the SDK. For example, the constant `STR_4` will get the `set_STR_4` method which accepts the same type defined in sway. Below is an example where we chain several `set` methods and execute the script with the new constants.

```rust,ignore
{{#include ../../packages/fuels/tests/configurables.rs:script_configurables}}
