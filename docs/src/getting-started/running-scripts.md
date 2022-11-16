# Running scripts

You can run a script using its JSON-ABI and the path to its binary file. You can run the scripts with arguments. For this, you have to use the `script_abigen!` macro, which is not unlike the `abigen!` macro seen [previously](../contracts/the-abigen-macro.md).

````rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_arguments}}
````
