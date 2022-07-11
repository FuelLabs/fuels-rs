# Converting native types

You might want to convert between the native types (`Bytes32`, `Address`, `ContractId`, and `AssetId`). Because these types are wrappers on `[u8; 32]`, converting is a matter of dereferencing one and instantiating the other using the dereferenced value. Here's an example:

```rust,ignore
{{#include ../../../examples/types/src/lib.rs:type_conversion}}
```
