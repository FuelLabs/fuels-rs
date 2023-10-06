# AssetId

Like `Bytes32`, `AssetId` is a wrapper on `[u8; 32]` with similar methods and implements the same traits (see [fuel-types documentation](https://docs.rs/fuel-types/{{versions.fuel-types}}/fuel_types/struct.AssetId.html)).

These are the main ways of creating an `AssetId`:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:asset_id}}
```
