# run_script 

`run_script` is helper function for testing simple Sway scripts and reducing boilerplate code related to setting up contracts and deployment.
It takes the path to the generated `.bin` file as argument.

You can use it this way:

````rust
{{#include ../../../packages/fuels-test-helpers/src/script.rs:test_logging_sway}}
````
