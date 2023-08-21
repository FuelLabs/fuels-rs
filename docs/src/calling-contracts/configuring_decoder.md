# Configuring the decoder

<!-- This section should explain what the call params are and how to configure them -->
<!-- call_params:example:start -->
You can also configure the decoder used to decode the return value of the contract method:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:contract_decoder_config}}
```

More on configuring decoders can be found [here](../codec/decoding.md).
