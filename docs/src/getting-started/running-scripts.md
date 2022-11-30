# Running scripts

You can run a script using its JSON-ABI and the path to its binary file. You can run the scripts with arguments. For this, you have to use the `script_abigen!` macro, which is similar to the `abigen!` macro seen [previously](../contracts/the-abigen-macro.md).

````rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_arguments}}
````

# Running scripts with transaction parameters

The method for passing transaction parameters is the same as [with contracts](../calling-contracts/tx-params.md). As a reminder, the workflow would look like this:

```rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_tx_params}}
```
