# Running scripts

`run_compiled_script` is a helper function for testing simple Sway scripts and reducing boilerplate code related to setting up contracts and deployment. As arguments, it takes: 
- the path to the generated binary file (`.bin`)
- the transactions parameters as an Option
- the provider as an Option
- the script data as an Option

````rust,ignore
{{#include ../../../packages/fuels-test-helpers/src/script.rs:run_compiled_script}}
````

# Running scripts with arguments

You can also run the scripts with arguments. For this, the setup is a little heavier because you have to use the `script_abigen!` macro, which is not unlike the `abigen!` macro seen [previously](../contracts/the-abigen-macro.md).

````rust,ignore
{{#include ../../../packages/fuels/tests/harness.rs:script_with_arguments}}
```
