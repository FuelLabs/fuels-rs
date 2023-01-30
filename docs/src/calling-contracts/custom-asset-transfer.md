# Custom asset transfer

The SDK provides the option to transfer assets within the same transaction, when making a contract call. By using `add_custom_asset()` you specify the asset id, the amount, and the destination address:

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:add_custom_assets}}
```
