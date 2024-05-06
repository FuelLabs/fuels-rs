# Function selector

Whenever you call a contract method the SDK will generate a function selector according to the fuel specs which will be
used by the node to identify which method we wish to execute.

If, for whatever reason, you wish to generate the function selector yourself you can do so:

```rust,ignore
{{#include ../../../examples/debugging/src/lib.rs:example_fn_selector}}
```
