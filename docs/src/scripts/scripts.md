# Running scripts

`run_compiled_script` is a helper function for testing simple Sway scripts and reducing boilerplate code related to setting up contracts and deployment. As the argument, it takes the path to the generated binary file (`.bin`).

````rust,ignore
{{#include ../../../packages/fuels-test-helpers/src/script.rs:run_compiled_script}}
````
