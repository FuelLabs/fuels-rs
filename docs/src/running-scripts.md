# Running scripts

You can run a script using its JSON-ABI and the path to its binary file. You can run the scripts with arguments. For this, you have to use the `abigen!` macro seen [previously](./abigen/the-abigen-macro.md).

```rust,ignore
{{#include ../../e2e/tests/scripts.rs:script_with_arguments}}
```

Furthermore, if you need to separate submission from value retrieval for any reason, you can do so as follows:

```rust,ignore
{{#include ../../e2e/tests/scripts.rs:submit_response_script}}
```

## Running scripts with transaction policies

The method for passing transaction policies is the same as [with contracts](./calling-contracts/tx-policies.md). As a reminder, the workflow would look like this:

```rust,ignore
{{#include ../../e2e/tests/scripts.rs:script_with_tx_policies}}
```

## Logs

Script calls provide the same logging functions, `decode_logs()` and `decode_logs_with_type<T>()`, as contract calls. As a reminder, the workflow looks like this:

```rust,ignore
{{#include ../../e2e/tests/logs.rs:script_logs}}
```

## Calling contracts from scripts

Scripts use the same interfaces for setting external contracts as [contract methods](./calling-contracts/other-contracts.md).

Below is an example that uses `with_contracts(&[&contract_instance, ...])`.

```rust,ignore
{{#include ../../e2e/tests/logs.rs:external_contract}}
```

And this is an example that uses `with_contract_ids(&[&contract_id, ...])`.

```rust,ignore
{{#include ../../e2e/tests/logs.rs:external_contract_ids}}
```

## Configurable constants

Same as contracts, you can define `configurable` constants in `scripts` which can be changed during the script execution. Here is an example how the constants are defined.

```rust,ignore
{{#include ../../e2e/sway/scripts/script_configurables/src/main.sw}}
```

Each configurable constant will get a dedicated `with` method in the SDK. For example, the constant `STR_4` will get the `with_STR_4` method which accepts the same type defined in sway. Below is an example where we chain several `with` methods and execute the script with the new constants.

```rust,ignore
{{#include ../../e2e/tests/configurables.rs:script_configurables}}
```
