# Custom inputs and outputs

If you need to add specific inputs and outputs to contract calls, you can use the `with_inputs` and `with_outputs` methods.

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:add_custom_inputs_outputs}}
```

> **Note:** if custom inputs include coins that need to be signed, use the `add_signer` method to add the appropriate signer.
