# Function selector

Whenever you call a contract method the SDK will generate a function selector according to the fuel specs which will be
used by the node to identify which method we wish to execute.

If, for whatever reason, you wish to generate the function selector yourself you can do so:

```rust,ignore
{{#include ../../../examples/debugging/src/lib.rs:example_fn_selector}}
```

## If you don't have the `ParamType`

If you won't or can't run the `abigen!` macro and all you have is the JSON ABI of you contract, you can still get the function
selector, but you have to jump through an extra hoop to get the `ParamTypes`:

```rust,ignore
{{#include ../../../examples/debugging/src/lib.rs:example_fn_selector_json}}
```
