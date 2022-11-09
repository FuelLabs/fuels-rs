# Running scripts

To run a script whose binary is located at a certain path, you can use the `Script` struct as 
such:

````rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_from_binary_filepath}}
````

# Running scripts with arguments

You can also run the scripts with arguments. For this, the setup is a little heavier because you have to use the `script_abigen!` macro, which is not unlike the `abigen!` macro seen [previously](../contracts/the-abigen-macro.md).

````rust,ignore
{{#include ../../../packages/fuels/tests/scripts.rs:script_with_arguments}}
````
