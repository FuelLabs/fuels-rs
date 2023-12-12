# Custom asset transfer

<!-- This section should explain the `add_custom_asset()` method -->
<!-- transfer:example:start -->
The SDK provides the option to transfer assets within the same transaction, when making a contract call. By using `add_custom_asset()` you specify the asset ID, the amount, and the destination address:
<!-- transfer:example:end -->

```rust,ignore
{{#include ../../../examples/contracts/src/lib.rs:add_custom_assets}}
```
