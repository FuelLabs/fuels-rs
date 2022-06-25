## Running a Sway script

`run_compiled_script` is a helper function for testing simple Sway scripts and reducing  boilerplate  code  related to setting up contracts and deployment. It takes the path to the generated `.bin` file as argument.

- You can use it this way:

````rust,ignore
{{#include ../../../packages/fuels-test-helpers/src/script.rs:run_compiled_script}}
````
